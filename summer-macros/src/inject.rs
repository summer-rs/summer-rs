use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    AngleBracketedGenericArguments, GenericArgument, Meta, MetaList, PathArguments, Token, Type,
    TypePath,
};

fn inject_error_tip() -> syn::Error {
    syn::Error::new(
        Span::call_site(),
        "inject Service only support Named-field Struct",
    )
}

enum InjectableType {
    Option,
    Component {
        optional: bool,
        component_type: syn::Path,
    },
    LazyComponent {
        component_type: syn::Path,
    },
    Config(syn::Path),
    ComponentRef(syn::Path),
    ConfigRef(syn::Path),
    FuncCall(Box<syn::ExprCall>),
    PrototypeArg(Box<syn::Type>),
}

impl InjectableType {
    fn order(&self) -> u8 {
        match self {
            Self::Option => 0,
            Self::Component { .. } => 1,
            Self::LazyComponent { .. } => 1,
            Self::Config(_) => 2,
            Self::ComponentRef(_) => 3,
            Self::ConfigRef(_) => 4,
            Self::FuncCall(_) => 5,
            Self::PrototypeArg(_) => 6,
        }
    }

    fn is_arg(&self) -> bool {
        matches!(self, Self::PrototypeArg(_))
    }
}

enum InjectableAttr {
    Component,
    Config,
    FuncCall(syn::ExprCall),
}

struct Injectable {
    is_prototype: bool,
    ty: InjectableType,
    field_name: syn::Ident,
}

impl Injectable {
    fn new(field: syn::Field, is_prototype: bool) -> syn::Result<Self> {
        let ty = Self::compute_type(&field, is_prototype)?;
        let field_name = field.ident.ok_or_else(inject_error_tip)?;
        Ok(Self {
            is_prototype,
            ty,
            field_name,
        })
    }

    fn compute_type(field: &syn::Field, is_prototype: bool) -> syn::Result<InjectableType> {
        if let syn::Type::Path(path) = &field.ty {
            let ty = &path.path;
            let inject_attr = field
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("inject"));

            if let Some(inject_attr) = inject_attr {
                if let Meta::List(MetaList { tokens, .. }) = &inject_attr.meta {
                    let attr = syn::parse::<InjectableAttr>(tokens.clone().into())?;
                    return attr.make_type(ty);
                } else {
                    Err(syn::Error::new_spanned(
                inject_attr,
                "invalid inject definition, expected #[inject(component|config|func(args))]",
                    ))?;
                }
            }

            let last_path_segment = ty.segments.last().ok_or_else(inject_error_tip)?;

            let type_name = &last_path_segment.ident;
            if type_name == "LazyComponent" {
                return Ok(InjectableType::LazyComponent {
                    component_type: get_argument_type(&last_path_segment.arguments)?,
                });
            }
            if last_path_segment.ident == "ComponentRef" {
                return Ok(InjectableType::ComponentRef(get_argument_type(
                    &last_path_segment.arguments,
                )?));
            }
            if last_path_segment.ident == "ConfigRef" {
                return Ok(InjectableType::ConfigRef(get_argument_type(
                    &last_path_segment.arguments,
                )?));
            }
            if !is_prototype && last_path_segment.ident == "Option" {
                return Ok(InjectableType::Option);
            }
        }
        if is_prototype {
            Ok(InjectableType::PrototypeArg(Box::new(field.ty.clone())))
        } else {
            let field_name = &field
                .ident
                .clone()
                .map(|ident| ident.to_string())
                .ok_or_else(inject_error_tip)?;
            Err(syn::Error::new_spanned(
            field,
            format!(
                "{field_name} field missing inject definition, expected #[inject(component|config|func(args))]",
            )))
        }
    }
}

impl syn::parse::Parse for InjectableAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse::<syn::Path>()?;
        if name.is_ident("component") {
            return Ok(Self::Component);
        }
        if name.is_ident("config") {
            return Ok(Self::Config);
        }
        if name.is_ident("func") {
            input.parse::<Token![=]>()?;
            let func_call = input.parse::<syn::ExprCall>()?;
            return Ok(Self::FuncCall(func_call));
        }
        Err(syn::Error::new(
            Span::call_site(),
            "invalid inject definition, expected #[inject(component|config|func(args))]",
        ))
    }
}

impl InjectableAttr {
    fn make_type(self, ty: &syn::Path) -> syn::Result<InjectableType> {
        Ok(match self {
            Self::Component => {
                let last_path_segment = ty.segments.last().ok_or_else(inject_error_tip)?;

                let (optional, component_type) = if last_path_segment.ident == "Option" {
                    (true, get_argument_type(&last_path_segment.arguments)?)
                } else {
                    (false, ty.clone())
                };

                InjectableType::Component {
                    optional,
                    component_type,
                }
            }
            Self::Config => InjectableType::Config(ty.clone()),
            Self::FuncCall(func_call) => InjectableType::FuncCall(Box::new(func_call)),
        })
    }
}

impl ToTokens for Injectable {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            is_prototype,
            ty,
            field_name,
        } = self;
        match ty {
            InjectableType::Option => {
                tokens.extend(quote! {
                    let #field_name = None;
                });
            }
            InjectableType::Component {
                optional,
                component_type,
            } =>
            {
                #[allow(clippy::collapsible_else_if)]
                if *optional {
                    if *is_prototype {
                        tokens.extend(quote! {
                            let #field_name = ::summer::App::global().get_component::<#component_type>();
                        });
                    } else {
                        tokens.extend(quote! {
                            let #field_name = app.get_component::<#component_type>();
                        });
                    }
                } else {
                    if *is_prototype {
                        tokens.extend(quote! {
                            let #field_name = ::summer::App::global().try_get_component::<#component_type>()?;
                        });
                    } else {
                        tokens.extend(quote! {
                            let #field_name = app.try_get_component::<#component_type>()?;
                        });
                    }
                }
            }
            InjectableType::LazyComponent { component_type } => {
                // For lazy components, wrap the component type in LazyComponent
                // This allows circular dependencies by deferring resolution
                tokens.extend(quote! {
                    let #field_name = ::summer::plugin::LazyComponent::<#component_type>::new();
                });
            }
            InjectableType::Config(type_path) => {
                if *is_prototype {
                    tokens.extend(quote! {
                        let #field_name = ::summer::App::global().get_config::<#type_path>()?;
                    });
                } else {
                    tokens.extend(quote! {
                        let #field_name = app.get_config::<#type_path>()?;
                    });
                }
            }
            InjectableType::ComponentRef(type_path) => {
                if *is_prototype {
                    tokens.extend(quote! {
                        let #field_name = ::summer::App::global().try_get_component_ref::<#type_path>()?;
                    });
                } else {
                    tokens.extend(quote! {
                        let #field_name = app.try_get_component_ref::<#type_path>()?;
                    });
                }
            }
            InjectableType::ConfigRef(type_path) => {
                if *is_prototype {
                    tokens.extend(quote! {
                        let #field_name = ::summer::config::ConfigRef::new(::summer::App::global().get_config::<#type_path>()?);
                    });
                } else {
                    tokens.extend(quote! {
                        let #field_name = ::summer::config::ConfigRef::new(app.get_config::<#type_path>()?);
                    });
                }
            }
            InjectableType::FuncCall(func_call) => {
                tokens.extend(quote! {
                    let #field_name = #func_call;
                });
            }
            InjectableType::PrototypeArg(type_path) => {
                // as func args
                tokens.extend(quote! {
                    #field_name: #type_path
                });
            }
        }
    }
}

struct Service {
    generics: syn::Generics,
    ident: proc_macro2::Ident,
    attr: Option<ServiceAttr>,
    fields: Vec<Injectable>,
}

enum ServiceAttr {
    Grpc(syn::Path),
    Prototype(syn::LitStr),
}

impl Service {
    fn new(input: syn::DeriveInput) -> syn::Result<Self> {
        let syn::DeriveInput {
            attrs,
            ident,
            generics,
            data,
            ..
        } = input;
        let service_attr = attrs
            .iter()
            .find(|a| a.path().is_ident("service"))
            .and_then(|attr| attr.parse_args_with(Self::parse_service_attr).ok());

        let is_prototype = matches!(&service_attr, Some(ServiceAttr::Prototype(_)));
        let mut fields = if let syn::Data::Struct(data) = data {
            data.fields
                .into_iter()
                .map(|f| Injectable::new(f, is_prototype))
                .collect::<syn::Result<Vec<_>>>()?
        } else {
            return Err(inject_error_tip());
        };
        fields.sort_by_key(|f| f.ty.order());

        // Put FuncCall at the end
        Ok(Self {
            generics,
            ident,
            attr: service_attr,
            fields,
        })
    }
    fn parse_service_attr(input: syn::parse::ParseStream) -> syn::Result<ServiceAttr> {
        let mut grpc: Option<syn::Path> = None;
        let mut prototype: Option<syn::LitStr> = None;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;

            if input.peek(syn::Token![=]) {
                input.parse::<syn::Token![=]>()?;
                let value: syn::LitStr = input.parse()?;

                match ident.to_string().as_str() {
                    "grpc" => {
                        if grpc.is_some() || prototype.is_some() {
                            return Err(syn::Error::new_spanned(
                                ident,
                                "Only one of `grpc` or `prototype` is allowed",
                            ));
                        }
                        grpc = Some(value.parse()?);
                    }
                    "prototype" => {
                        if prototype.is_some() || grpc.is_some() {
                            return Err(syn::Error::new_spanned(
                                ident,
                                "Only one of `grpc` or `prototype` is allowed",
                            ));
                        }
                        prototype = Some(value);
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            ident,
                            format!("Unknown key `{other}` in #[service(...)], expected `grpc` or `prototype`"),
                        ));
                    }
                }
            } else {
                // 标志形式：#[service(prototype)]
                match ident.to_string().as_str() {
                    "prototype" => {
                        if prototype.is_some() || grpc.is_some() {
                            return Err(syn::Error::new_spanned(
                                ident,
                                "Only one of `grpc` or `prototype` is allowed",
                            ));
                        }
                        prototype = Some(syn::LitStr::new("build", Span::call_site()));
                        // 默认build
                    }
                    "grpc" => {
                        return Err(syn::Error::new_spanned(
                            ident,
                            "`grpc` must have a value like `grpc = \"...\"`",
                        ));
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            ident,
                            format!("Unknown key `{other}` in #[service(...)]"),
                        ));
                    }
                }
            }

            // 跳过逗号
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        match (grpc, prototype) {
            (Some(path), None) => Ok(ServiceAttr::Grpc(path)),
            (None, Some(litstr_opt)) => Ok(ServiceAttr::Prototype(litstr_opt)),
            (None, None) => Err(syn::Error::new(
                input.span(),
                "Expected at least one of `grpc` or `prototype`",
            )),
            _ => unreachable!(),
        }
    }
}

impl ToTokens for Service {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            generics,
            ident,
            attr,
            fields,
        } = self;
        let field_names: Vec<&syn::Ident> = fields.iter().map(|f| &f.field_name).collect();

        let output = match attr {
            Some(ServiceAttr::Prototype(build)) => {
                let fn_name = syn::Ident::new(&build.value(), build.span());
                let (args, fields): (Vec<&Injectable>, Vec<&Injectable>) =
                    fields.iter().partition(|f| f.ty.is_arg());
                let syn::Generics {
                    lt_token,
                    params,
                    gt_token,
                    ..
                } = generics;
                quote! {
                    impl #lt_token #params #gt_token #ident #generics {
                        pub fn #fn_name(#(#args),*) -> ::summer::error::Result<Self> {
                            use ::summer::plugin::ComponentRegistry;
                            use ::summer::config::ConfigRegistry;
                            #(#fields)*
                            Ok(Self { #(#field_names),* })
                        }
                    }
                }
            }
            _ => {
                let service_registrar =
                    syn::Ident::new(&format!("__ServiceRegistrarFor_{ident}"), ident.span());
                let service_installer = match attr {
                    Some(ServiceAttr::Grpc(server)) => {
                        quote! {
                            use ::summer::plugin::MutableComponentRegistry;
                            use ::summer_grpc::GrpcConfigurator;
                            let service = #ident::build(app)?;
                            let grpc_server = #server::new(service.clone());
                            app.add_component(service).add_service(grpc_server);
                        }
                    }
                    _ => {
                        quote! {
                            use ::summer::plugin::MutableComponentRegistry;
                            app.add_component(#ident::build(app)?);
                        }
                    }
                };
                quote! {
                    impl ::summer::plugin::service::Service for #ident {
                        fn build<R>(app: &R) -> ::summer::error::Result<Self>
                        where
                            R: ::summer::plugin::ComponentRegistry + ::summer::config::ConfigRegistry
                        {
                            #(#fields)*
                            Ok(Self { #(#field_names),* })
                        }
                    }
                    #[allow(non_camel_case_types)]
                    struct #service_registrar;
                    impl ::summer::plugin::service::ServiceRegistrar for #service_registrar{
                        fn install_service(&self, app: &mut ::summer::app::AppBuilder)->::summer::error::Result<()> {
                            #service_installer
                            Ok(())
                        }
                    }
                    ::summer::submit_service!(#service_registrar);
                }
            }
        };
        tokens.extend(output);
    }
}

fn get_argument_type(path_args: &PathArguments) -> syn::Result<syn::Path> {
    if let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) = path_args {
        let ty = args.last().ok_or_else(inject_error_tip)?;
        if let GenericArgument::Type(Type::Path(TypePath { path, .. })) = ty {
            return Ok(path.clone());
        }
    }
    Err(inject_error_tip())
}

pub(crate) fn expand_derive(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    Ok(Service::new(input)?.into_token_stream())
}
