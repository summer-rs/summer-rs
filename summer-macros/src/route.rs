mod openapi;

use crate::{input_and_compile_error, utils};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use std::collections::HashSet;
use syn::{punctuated::Punctuated, LitStr, Path, Token};

macro_rules! standard_http_method {
    (
        $($variant:ident, $upper:ident, $lower:ident,)+
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum Method {
            $(
                $variant,
            )+
        }

        impl Method {
            fn parse(method: &str) -> Result<Self, String> {
                match method {
                    $(stringify!($upper) => Ok(Self::$variant),)+
                    _ => Err(format!("HTTP method must be uppercase: `{}`", method)),
                }
            }

            pub(crate) fn from_path(method: &Path) -> Result<Self, ()> {
                match () {
                    $(_ if method.is_ident(stringify!($lower)) => Ok(Self::$variant),)+
                    _ => Err(()),
                }
            }

            pub(crate) fn as_lowercase_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($lower),)+
                }
            }
        }

        impl ToTokens for Method {
            fn to_tokens(&self, output: &mut TokenStream2) {
                let stream = match self {
                    $(Self::$variant => quote!(::summer_web::MethodFilter::$upper),)+
                };

                output.extend(stream);
            }
        }
    };
}

standard_http_method! {
    Get,       GET,     get,
    Post,      POST,    post,
    Put,       PUT,     put,
    Delete,    DELETE,  delete,
    Head,      HEAD,    head,
    Options,   OPTIONS, options,
    Trace,     TRACE,   trace,
    Patch,     PATCH,   patch,
}

impl TryFrom<&syn::LitStr> for Method {
    type Error = syn::Error;

    fn try_from(value: &syn::LitStr) -> Result<Self, Self::Error> {
        Self::parse(value.value().as_str())
            .map_err(|message| syn::Error::new_spanned(value, message))
    }
}

#[derive(Debug)]
pub(crate) struct RouteArgs {
    pub(crate) path: LitStr,
    pub(crate) options: Punctuated<syn::Meta, Token![,]>,
}

impl syn::parse::Parse for RouteArgs {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        // path to match: "/foo"
        let path = input.parse::<syn::LitStr>().map_err(|mut err| {
            err.combine(syn::Error::new(
                err.span(),
                r#"invalid route definition, expected #[<method>("<path>")]"#,
            ));

            err
        })?;

        // verify that path pattern is valid
        //let _ = ResourceDef::new(path.value());

        // if there's no comma, assume that no options are provided
        if !input.peek(Token![,]) {
            return Ok(Self {
                path,
                options: Punctuated::new(),
            });
        }

        // advance past comma separator
        input.parse::<Token![,]>()?;

        // if next char is a literal, assume that it is a string and show multi-path error
        if input.cursor().literal().is_some() {
            return Err(syn::Error::new(
                Span::call_site(),
                r#"Multiple paths specified! There should be only one."#,
            ));
        }

        // zero or more options: name = "foo"
        let options = input.parse_terminated(syn::Meta::parse, Token![,])?;

        Ok(Self { path, options })
    }
}

struct Args {
    path: syn::LitStr,
    methods: HashSet<Method>,
    debug: bool,
    transform: Option<syn::ExprPath>,
}

impl Args {
    fn new(args: RouteArgs, method: Option<Method>) -> syn::Result<Self> {
        let mut methods = HashSet::new();

        let is_route_macro: bool = method.is_none();
        if let Some(method) = method {
            methods.insert(method);
        }
        let mut debug = false;
        let mut transform = None;
        for meta in args.options {
            match meta {
                syn::Meta::Path(path) if path.is_ident("debug") => {
                    debug = true;
                }
                syn::Meta::NameValue(nv) => {
                    if nv.path.is_ident("method") {
                        if !is_route_macro {
                            return Err(syn::Error::new_spanned(
                                &nv,
                                "HTTP method forbidden here; to handle multiple methods, use `route` instead",
                            ));
                        } else if let syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(lit),
                            ..
                        }) = nv.value.clone()
                        {
                            if !methods.insert(Method::try_from(&lit)?) {
                                return Err(syn::Error::new_spanned(
                                    nv.value,
                                    format!(
                                        "HTTP method defined more than once: `{}`",
                                        lit.value()
                                    ),
                                ));
                            }
                        } else {
                            return Err(syn::Error::new_spanned(
                                nv.value,
                                "Attribute method expects literal string",
                            ));
                        }
                    } else if nv.path.is_ident("transform") {
                        if let syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(lit),
                            ..
                        }) = nv.value.clone()
                        {
                            let expr: syn::ExprPath = syn::parse_str(&lit.value())?;
                            transform = Some(expr);
                        } else {
                            return Err(syn::Error::new_spanned(
                                nv.value,
                                "transform expects string literal path",
                            ));
                        }
                    } else {
                        let attr = nv.path.to_token_stream();
                        return Err(syn::Error::new_spanned(
                            nv,
                            format!(
                                "Unknown attribute `{attr}`; allowed: `method = \"METHOD\"`, `transform = \"path::to::fn\"`, `debug`"
                            ),
                        ));
                    }
                }
                other => {
                    let attr = other.path().to_token_stream();
                    return Err(syn::Error::new_spanned(
                        other,
                        format!(
                            "Unknown attribute `{attr}`; allowed: `method = \"METHOD\"`, `debug`"
                        ),
                    ));
                }
            }
        }

        Ok(Args {
            path: args.path,
            methods,
            debug,
            transform,
        })
    }
}

struct Route {
    /// Name of the handler function being annotated.
    name: syn::Ident,

    /// Args passed to routing macro.
    ///
    /// When using `#[routes]`, this will contain args for each specific routing macro.
    args: Vec<Args>,

    /// AST of the handler function being annotated.
    ast: syn::ItemFn,

    /// The doc comment attributes to copy to generated struct, if any.
    doc_attributes: Vec<syn::Attribute>,

    /// Whether to apply `#[axum::debug_handler]`
    debug: bool,

    /// Whether to enable `openapi`
    openapi: bool,
}

impl Route {
    pub fn new(
        args: RouteArgs,
        ast: syn::ItemFn,
        method: Option<Method>,
        openapi: bool,
    ) -> syn::Result<Self> {
        let name = ast.sig.ident.clone();

        // Try and pull out the doc comments so that we can reapply them to the generated struct.
        // Note that multi line doc comments are converted to multiple doc attributes.
        let doc_attributes = ast
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("doc"))
            .cloned()
            .collect();

        let args = Args::new(args, method)?;
        let debug = args.debug;

        if args.methods.is_empty() {
            return Err(syn::Error::new(
                Span::call_site(),
                "The #[route(..)] macro requires at least one `method` attribute",
            ));
        }

        if matches!(ast.sig.output, syn::ReturnType::Default) {
            return Err(syn::Error::new_spanned(
                ast,
                "Function has no return type. Cannot be used as handler",
            ));
        }

        if ast.sig.asyncness.is_none() {
            return Err(syn::Error::new_spanned(
                ast.sig.fn_token,
                "only support async fn",
            ));
        }

        Ok(Self {
            name,
            args: vec![args],
            ast,
            doc_attributes,
            debug,
            openapi,
        })
    }

    /// routers
    fn multiple(args: Vec<Args>, ast: syn::ItemFn, openapi: bool) -> syn::Result<Self> {
        let debug = args.iter().any(|a| a.debug);
        let name = ast.sig.ident.clone();

        // Try and pull out the doc comments so that we can reapply them to the generated struct.
        // Note that multi line doc comments are converted to multiple doc attributes.
        let doc_attributes = ast
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("doc"))
            .cloned()
            .collect();

        if matches!(ast.sig.output, syn::ReturnType::Default) {
            return Err(syn::Error::new_spanned(
                ast,
                "Function has no return type. Cannot be used as handler",
            ));
        }

        Ok(Self {
            name,
            args,
            ast,
            doc_attributes,
            debug,
            openapi,
        })
    }
}

impl ToTokens for Route {
    fn to_tokens(&self, output: &mut TokenStream2) {
        let Self {
            name,
            ast,
            args,
            doc_attributes,
            debug,
            openapi,
        } = self;

        let vis = &ast.vis;

        let registrations = args
            .iter()
            .map(|args| {
                let Args { path, methods, transform,.. } = args;

                if *openapi {
                    let fn_name = name.to_string();
                    let operation = openapi::parse_doc_attributes(doc_attributes, &fn_name);
                    let status_codes = &operation.status_codes;
                    
                    let status_code_gen = if !status_codes.is_empty() {
                        let registrations = status_codes.iter().map(|variant_path| {
                            let path_parts: Vec<&str> = variant_path.split("::").collect();
                            if path_parts.len() < 2 {
                                panic!("Invalid status_codes format: {}. Expected format: TypeName::VariantName", variant_path);
                            }
                            
                            let type_path_parts = &path_parts[..path_parts.len() - 1];
                            let type_path_str = type_path_parts.join("::");
                            
                            let type_path = syn::parse_str::<syn::Path>(&type_path_str)
                                .unwrap_or_else(|_| panic!("Invalid type path: {}", type_path_str));
                            
                            quote! {
                                {
                                    ::summer_web::openapi::register_error_response_by_variant::<#type_path>(
                                        ctx,
                                        &mut __operation,
                                        #variant_path
                                    );
                                }
                            }
                        });
                        quote! {
                            #(#registrations)*
                        }
                    } else {
                        quote! {}
                    };
                    
                    let (input_tys, output_ty) = utils::extract_fn_types(ast);
                    let gen_output = if let Some(ty) = output_ty {
                        quote! {
                            for (code, res) in <#ty as ::summer_web::aide::OperationOutput>::inferred_responses(ctx, &mut __operation) {
                                ::summer_web::openapi::set_inferred_response(ctx, &mut __operation, code, res);
                            }
                        }
                    } else {
                        quote! {}
                    };
                    let method_binder = methods
                        .iter()
                        .map(|m| quote! {let __method_router=::summer_web::MethodRouter::on(__method_router, #m, #name);});
                    let operation_binder = methods
                        .iter()
                        .map(|m| {
                            let method_str = m.as_lowercase_str();
                            quote! {
                                let mut __operation = #operation;
                                ::summer_web::aide::generate::in_context(|ctx| {
                                    #(
                                        <#input_tys as ::summer_web::aide::OperationInput>::operation_input(ctx, &mut __operation);
                                    )*
                                    #gen_output
                                    #status_code_gen
                                });
                                __router = __router.api_route_docs_with(#path, ::summer_web::aide::axum::routing::ApiMethodDocs::new(#method_str, __operation), __transform);
                            }
                        });
                    let transform_ts = if let Some(t) = transform {
                        quote! { let __transform = #t; }
                    } else {
                        quote! {
                            let __transform = ::summer_web::default_transform;
                        }
                    };
                    quote! {
                        let __method_router = ::summer_web::MethodRouter::new();
                        #(#method_binder)*
                        let __method_router = ::summer_web::ApiMethodRouter::from(__method_router);
                        __router = ::summer_web::Router::api_route(__router, #path, __method_router);
                        #transform_ts
                        #(#operation_binder)*
                    }
                } else {
                    let method_binder = methods
                        .iter()
                        .map(|m| quote! {let __method_router=::summer_web::MethodRouter::on(__method_router, #m, #name);});

                    quote! {
                        let __method_router = ::summer_web::MethodRouter::new();
                        #(#method_binder)*
                        __router = ::summer_web::Router::route(__router, #path, __method_router);
                    }
                }
            });
        let handler_fn = if *debug {
            let sig = &ast.sig;
            let vis = &ast.vis;
            let attrs = &ast.attrs;
            let block = &ast.block;

            quote! {
                #[::summer_web::axum::debug_handler]
                #(#attrs)*
                #vis #sig #block
            }
        } else {
            quote! { #ast }
        };

        let stream = quote! {
            #(#doc_attributes)*
            #[allow(non_camel_case_types, missing_docs)]
            #vis struct #name;

            impl ::summer_web::handler::TypedHandlerRegistrar for #name {
                fn install_route(&self, mut __router: ::summer_web::Router) -> ::summer_web::Router{
                    #handler_fn
                    #(#registrations)*

                    __router
                }
            }

            ::summer_web::submit_typed_handler!(#name);
        };

        output.extend(stream);
    }
}

pub(crate) fn with_method(
    method: Option<Method>,
    args: TokenStream,
    input: TokenStream,
    openapi: bool,
) -> TokenStream {
    let args = match syn::parse(args) {
        Ok(args) => args,
        // on parse error, make IDEs happy; see fn docs
        Err(err) => return input_and_compile_error(input, err),
    };

    let ast = match syn::parse::<syn::ItemFn>(input.clone()) {
        Ok(ast) => ast,
        // on parse error, make IDEs happy; see fn docs
        Err(err) => return input_and_compile_error(input, err),
    };

    match Route::new(args, ast, method, openapi) {
        Ok(route) => route.into_token_stream().into(),
        // on macro related error, make IDEs happy; see fn docs
        Err(err) => input_and_compile_error(input, err),
    }
}

pub(crate) fn with_methods(input: TokenStream, openapi: bool) -> TokenStream {
    let mut ast = match syn::parse::<syn::ItemFn>(input.clone()) {
        Ok(ast) => ast,
        // on parse error, make IDEs happy; see fn docs
        Err(err) => return input_and_compile_error(input, err),
    };

    let (methods, others) = ast
        .attrs
        .into_iter()
        .map(|attr| match Method::from_path(attr.path()) {
            Ok(method) => Ok((method, attr)),
            Err(_) => Err(attr),
        })
        .partition::<Vec<_>, _>(Result::is_ok);

    ast.attrs = others.into_iter().map(Result::unwrap_err).collect();

    let methods = match methods
        .into_iter()
        .map(Result::unwrap)
        .map(|(method, attr)| {
            attr.parse_args()
                .and_then(|args| Args::new(args, Some(method)))
        })
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(methods) if methods.is_empty() => {
            return input_and_compile_error(
                input,
                syn::Error::new(
                    Span::call_site(),
                    "The #[routes] macro requires at least one `#[<method>(..)]` attribute.",
                ),
            )
        }
        Ok(methods) => methods,
        Err(err) => return input_and_compile_error(input, err),
    };

    match Route::multiple(methods, ast, openapi) {
        Ok(route) => route.into_token_stream().into(),
        // on macro related error, make IDEs happy; see fn docs
        Err(err) => input_and_compile_error(input, err),
    }
}
