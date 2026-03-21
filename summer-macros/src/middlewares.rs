use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, Expr, Token, parse::Parse};

// ── Single source of truth for route attribute definitions ───────────────────

struct RouteAttrDef {
    name: &'static str,
    http_method: Option<&'static str>,
    openapi: bool,
}

const ROUTE_ATTR_DEFS: &[RouteAttrDef] = &[
    RouteAttrDef { name: "get",         http_method: Some("GET"),     openapi: false },
    RouteAttrDef { name: "post",        http_method: Some("POST"),    openapi: false },
    RouteAttrDef { name: "put",         http_method: Some("PUT"),     openapi: false },
    RouteAttrDef { name: "delete",      http_method: Some("DELETE"),  openapi: false },
    RouteAttrDef { name: "patch",       http_method: Some("PATCH"),   openapi: false },
    RouteAttrDef { name: "head",        http_method: Some("HEAD"),    openapi: false },
    RouteAttrDef { name: "options",     http_method: Some("OPTIONS"), openapi: false },
    RouteAttrDef { name: "trace",       http_method: Some("TRACE"),   openapi: false },
    RouteAttrDef { name: "get_api",     http_method: Some("GET"),     openapi: true },
    RouteAttrDef { name: "post_api",    http_method: Some("POST"),    openapi: true },
    RouteAttrDef { name: "put_api",     http_method: Some("PUT"),     openapi: true },
    RouteAttrDef { name: "delete_api",  http_method: Some("DELETE"),  openapi: true },
    RouteAttrDef { name: "patch_api",   http_method: Some("PATCH"),   openapi: true },
    RouteAttrDef { name: "head_api",    http_method: Some("HEAD"),    openapi: true },
    RouteAttrDef { name: "options_api", http_method: Some("OPTIONS"), openapi: true },
    RouteAttrDef { name: "trace_api",   http_method: Some("TRACE"),   openapi: true },
    RouteAttrDef { name: "route",       http_method: None,            openapi: false },
    RouteAttrDef { name: "routes",      http_method: None,            openapi: false },
    RouteAttrDef { name: "api_route",   http_method: None,            openapi: true },
    RouteAttrDef { name: "api_routes",  http_method: None,            openapi: true },
];

fn lookup_attr(attr: &syn::Attribute) -> Option<&'static RouteAttrDef> {
    ROUTE_ATTR_DEFS.iter().find(|def| attr.path().is_ident(def.name))
}

fn is_route_attr(attr: &syn::Attribute) -> bool {
    lookup_attr(attr).is_some()
}

fn is_openapi_attr(attr: &syn::Attribute) -> bool {
    lookup_attr(attr).is_some_and(|def| def.openapi)
}

fn attr_to_http_method(attr: &syn::Attribute) -> Option<&'static str> {
    lookup_attr(attr).and_then(|def| def.http_method)
}

// ── Parsing helpers ─────────────────────────────────────────────────────────

struct MiddlewareList {
    middlewares: Punctuated<Expr, Token![,]>,
}

impl Parse for MiddlewareList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let middlewares = Punctuated::<Expr, Token![,]>::parse_terminated(input)?;
        Ok(MiddlewareList { middlewares })
    }
}

fn extract_and_filter_route_attrs(attrs: &mut Vec<syn::Attribute>) -> Vec<syn::Attribute> {
    let mut route_attrs = Vec::new();
    attrs.retain(|attr| {
        if is_route_attr(attr) {
            route_attrs.push(attr.clone());
            false
        } else {
            !attr.path().is_ident("middlewares")
        }
    });
    route_attrs
}

fn extract_nest_prefix(module: &syn::ItemMod) -> syn::Result<Option<String>> {
    for attr in &module.attrs {
        if attr.path().is_ident("nest") {
            if let Ok(path_lit) = attr.parse_args::<syn::LitStr>() {
                return Ok(Some(path_lit.value()));
            }
        }
    }
    Ok(None)
}

fn extract_function_middlewares(attrs: &[syn::Attribute]) -> syn::Result<Vec<syn::Expr>> {
    for attr in attrs {
        if attr.path().is_ident("middlewares") {
            let middleware_list = attr.parse_args::<MiddlewareList>()?;
            return Ok(middleware_list.middlewares.into_iter().collect());
        }
    }
    Ok(Vec::new())
}

fn extract_doc_attributes(attrs: &[syn::Attribute]) -> Vec<syn::Attribute> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .cloned()
        .collect()
}

fn remove_processed_attributes(attrs: &mut Vec<syn::Attribute>) {
    attrs.retain(|attr| !attr.path().is_ident("middlewares") && !is_route_attr(attr));
}

// ── RouteInfo ───────────────────────────────────────────────────────────────

struct RouteInfo {
    func_name: syn::Ident,
    path: String,
    methods: Vec<String>,
    function_middlewares: Vec<syn::Expr>,
    is_openapi: bool,
    doc_attributes: Vec<syn::Attribute>,
    fn_ast: Option<syn::ItemFn>,
}

fn process_route_attribute(
    attr: &syn::Attribute,
    func_name: &syn::Ident,
    function_middlewares: &[syn::Expr],
    doc_attributes: &[syn::Attribute],
    fn_ast: Option<&syn::ItemFn>,
) -> syn::Result<Option<RouteInfo>> {
    let Some(method) = attr_to_http_method(attr) else {
        return Ok(None);
    };
    let Ok(path_lit) = attr.parse_args::<syn::LitStr>() else {
        return Ok(None);
    };
    let is_openapi = is_openapi_attr(attr);

    Ok(Some(RouteInfo {
        func_name: func_name.clone(),
        path: path_lit.value(),
        methods: vec![method.to_string()],
        function_middlewares: function_middlewares.to_vec(),
        is_openapi,
        doc_attributes: if is_openapi { doc_attributes.to_vec() } else { vec![] },
        fn_ast: if is_openapi { fn_ast.cloned() } else { None },
    }))
}

fn collect_and_strip_route_info(
    module: &mut syn::ItemMod,
    _nest_prefix: Option<&str>,
) -> syn::Result<Vec<RouteInfo>> {
    let mut route_info = Vec::new();

    let Some((_, items)) = &mut module.content else {
        return Ok(route_info);
    };

    for item in items {
        if let syn::Item::Fn(fun) = item {
            let function_middlewares = extract_function_middlewares(&fun.attrs)?;
            let doc_attributes = extract_doc_attributes(&fun.attrs);
            let has_openapi = fun.attrs.iter().any(is_openapi_attr);
            let fn_ast_snapshot = if has_openapi { Some(fun.clone()) } else { None };
            let route_attrs = extract_and_filter_route_attrs(&mut fun.attrs);

            for attr in route_attrs {
                if let Some(route) = process_route_attribute(
                    &attr,
                    &fun.sig.ident,
                    &function_middlewares,
                    &doc_attributes,
                    fn_ast_snapshot.as_ref(),
                )? {
                    route_info.push(route);
                }
            }
        }
    }

    Ok(route_info)
}

fn extract_route_info_from_function(function: &syn::ItemFn) -> syn::Result<Vec<RouteInfo>> {
    let mut route_info = Vec::new();
    let doc_attributes = extract_doc_attributes(&function.attrs);

    for attr in &function.attrs {
        let fn_ref = if is_openapi_attr(attr) { Some(function) } else { None };
        if let Some(route) = process_route_attribute(
            attr,
            &function.sig.ident,
            &[],
            &doc_attributes,
            fn_ref,
        )? {
            route_info.push(route);
        }
    }

    Ok(route_info)
}

// ── OpenAPI codegen (shared between module-level & function-level) ───────────

struct OpenApiCodegen {
    status_code_gen: TokenStream2,
    input_types_gen: TokenStream2,
    output_gen: TokenStream2,
}

impl OpenApiCodegen {
    fn from_route(route: &RouteInfo) -> Self {
        let status_code_gen = Self::build_status_code_gen(route);
        let (input_types_gen, output_gen) = Self::build_type_gens(route);
        Self { status_code_gen, input_types_gen, output_gen }
    }

    fn build_status_code_gen(route: &RouteInfo) -> TokenStream2 {
        let fn_name_str = route.func_name.to_string();
        let operation = crate::route::openapi::parse_doc_attributes(&route.doc_attributes, &fn_name_str);
        let status_codes = &operation.status_codes;

        if status_codes.is_empty() {
            return quote! {};
        }

        let registrations = status_codes.iter().map(|variant_path| {
            let path_parts: Vec<&str> = variant_path.split("::").collect();
            if path_parts.len() < 2 {
                panic!(
                    "Invalid status_codes format: {}. Expected format: TypeName::VariantName",
                    variant_path
                );
            }
            let type_path_str = path_parts[..path_parts.len() - 1].join("::");
            let type_path = syn::parse_str::<syn::Path>(&type_path_str)
                .unwrap_or_else(|_| panic!("Invalid type path: {}", type_path_str));
            quote! {
                ::summer_web::openapi::register_error_response_by_variant::<#type_path>(
                    ctx, &mut __operation, #variant_path
                );
            }
        });
        quote! { #(#registrations)* }
    }

    fn build_type_gens(route: &RouteInfo) -> (TokenStream2, TokenStream2) {
        let Some(ast) = &route.fn_ast else {
            return (quote! {}, quote! {});
        };
        let (input_tys, output_ty) = crate::utils::extract_fn_types(ast);

        let input_gen = if input_tys.is_empty() {
            quote! {}
        } else {
            quote! {
                #(<#input_tys as ::summer_web::aide::OperationInput>::operation_input(ctx, &mut __operation);)*
            }
        };

        let output_gen = match output_ty {
            Some(ty) => quote! {
                for (code, res) in <#ty as ::summer_web::aide::OperationOutput>::inferred_responses(ctx, &mut __operation) {
                    ::summer_web::openapi::set_inferred_response(ctx, &mut __operation, code, res);
                }
            },
            None => quote! {},
        };

        (input_gen, output_gen)
    }

    /// Generate the `api_route_docs_with` binding tokens for a given router ident.
    fn operation_binder_tokens(
        &self,
        route: &RouteInfo,
        router_ident: &syn::Ident,
    ) -> Vec<TokenStream2> {
        let path = &route.path;
        let fn_name_str = route.func_name.to_string();
        let operation = crate::route::openapi::parse_doc_attributes(&route.doc_attributes, &fn_name_str);
        let Self { status_code_gen, input_types_gen, output_gen } = self;

        route
            .methods
            .iter()
            .map(|m| m.to_lowercase())
            .map(|method_str| {
                quote! {
                    let mut __operation = #operation;
                    ::summer_web::aide::generate::in_context(|ctx| {
                        #input_types_gen
                        #output_gen
                        #status_code_gen
                    });
                    #router_ident = #router_ident.api_route_docs_with(
                        #path,
                        ::summer_web::aide::axum::routing::ApiMethodDocs::new(#method_str, __operation),
                        __transform,
                    );
                }
            })
            .collect()
    }
}

// ── Route registration codegen ──────────────────────────────────────────────

fn generate_method_filters(methods: &[String]) -> Vec<TokenStream2> {
    methods
        .iter()
        .map(|method_str| {
            let method_ident = syn::Ident::new(method_str, Span::call_site());
            quote! { ::summer_web::MethodFilter::#method_ident }
        })
        .collect()
}

/// Build the method-router creation tokens shared by all route types.
fn build_method_router_tokens(route: &RouteInfo) -> TokenStream2 {
    let func_name = &route.func_name;
    let methods = generate_method_filters(&route.methods);
    quote! {
        let __method_router = ::summer_web::MethodRouter::new();
        #(let __method_router = ::summer_web::MethodRouter::on(__method_router, #methods, #func_name);)*
    }
}

/// Generate route registration for module-level middleware context.
/// Routes are registered onto `__module_router`.
fn generate_route_registration(route: &RouteInfo) -> TokenStream2 {
    let path = &route.path;
    let method_router = build_method_router_tokens(route);
    let fn_mw_layers = reverse_middleware_layers(&route.function_middlewares);

    if route.is_openapi {
        let codegen = OpenApiCodegen::from_route(route);
        let router_ident = syn::Ident::new("__module_router", Span::call_site());
        let op_binders = codegen.operation_binder_tokens(route, &router_ident);

        if fn_mw_layers.is_empty() {
            quote! {
                #method_router
                let __method_router = ::summer_web::ApiMethodRouter::from(__method_router);
                __module_router = ::summer_web::Router::api_route(__module_router, #path, __method_router);
                let __transform = ::summer_web::default_transform;
                #(#op_binders)*
            }
        } else {
            quote! {
                let mut __function_router = ::summer_web::Router::new();
                #method_router
                let __method_router = ::summer_web::ApiMethodRouter::from(__method_router);
                __function_router = ::summer_web::Router::api_route(__function_router, #path, __method_router);
                let __transform = ::summer_web::default_transform;
                #(#op_binders)*
                #(let __function_router = __function_router.layer(#fn_mw_layers);)*
                __module_router = __module_router.merge(__function_router);
            }
        }
    } else if fn_mw_layers.is_empty() {
        quote! {
            #method_router
            __module_router = ::summer_web::Router::route(__module_router, #path, __method_router);
        }
    } else {
        quote! {
            let mut __function_router = ::summer_web::Router::new();
            #method_router
            __function_router = ::summer_web::Router::route(__function_router, #path, __method_router);
            #(let __function_router = __function_router.layer(#fn_mw_layers);)*
            __module_router = __module_router.merge(__function_router);
        }
    }
}

/// Generate route registration for function-level middleware context.
/// Routes are registered onto `__router`, middleware is applied per-method-router.
fn generate_function_route_registration(
    route: &RouteInfo,
    func_name: &syn::Ident,
    middleware_expressions: &[TokenStream2],
) -> TokenStream2 {
    let path = &route.path;
    let methods = generate_method_filters(&route.methods);

    if route.is_openapi {
        let codegen = OpenApiCodegen::from_route(route);
        let router_ident = syn::Ident::new("__router", Span::call_site());
        let op_binders = codegen.operation_binder_tokens(route, &router_ident);

        quote! {
            let mut __method_router = ::summer_web::MethodRouter::new();
            #(let __method_router = ::summer_web::MethodRouter::on(__method_router, #methods, #func_name);)*
            #(let __method_router = __method_router.layer(#middleware_expressions);)*
            let __method_router = ::summer_web::ApiMethodRouter::from(__method_router);
            __router = ::summer_web::Router::api_route(__router, #path, __method_router);
            let __transform = ::summer_web::default_transform;
            #(#op_binders)*
        }
    } else {
        quote! {
            let mut __method_router = ::summer_web::MethodRouter::new();
            #(let __method_router = ::summer_web::MethodRouter::on(__method_router, #methods, #func_name);)*
            #(let __method_router = __method_router.layer(#middleware_expressions);)*
            __router = ::summer_web::Router::route(__router, #path, __method_router);
        }
    }
}

fn reverse_middleware_layers(middlewares: &[syn::Expr]) -> Vec<TokenStream2> {
    middlewares.iter().rev().map(|mw| quote! { #mw }).collect()
}

// ── Entry point & top-level handlers ────────────────────────────────────────

pub fn middlewares(args: TokenStream, input: TokenStream) -> TokenStream {
    match middlewares_inner(args, input.clone()) {
        Ok(stream) => stream,
        Err(err) => crate::input_and_compile_error(input, err),
    }
}

fn middlewares_inner(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    if args.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "missing arguments for middlewares macro, expected: #[middlewares(middleware1, middleware2, ...)]",
        ));
    }

    let middleware_list = syn::parse::<MiddlewareList>(args.clone()).map_err(|err| {
        syn::Error::new(
            err.span(),
            "arguments to middlewares macro must be valid expressions, expected: #[middlewares(middleware1, middleware2, ...)]",
        )
    })?;

    if let Ok(function) = syn::parse::<syn::ItemFn>(input.clone()) {
        return handle_function_middlewares(&middleware_list.middlewares, &function);
    }

    handle_module_middlewares(middleware_list, input)
}

fn handle_module_middlewares(
    middleware_list: MiddlewareList,
    input: TokenStream,
) -> syn::Result<TokenStream> {
    let mut module = syn::parse::<syn::ItemMod>(input).map_err(|err| {
        syn::Error::new(err.span(), "#[middlewares] macro must be attached to a module")
    })?;

    let module_name = &module.ident;
    let registrar_struct_name =
        syn::Ident::new(&format!("{module_name}MiddlewareRegistrar"), module.ident.span());

    let nest_prefix = extract_nest_prefix(&module)?;
    let route_info = collect_and_strip_route_info(&mut module, nest_prefix.as_deref())?;

    let route_registrations: Vec<_> = route_info.iter().map(generate_route_registration).collect();
    let middleware_expressions: Vec<_> = middleware_list
        .middlewares
        .iter()
        .rev()
        .map(|mw| quote! { #mw })
        .collect();

    add_registrar_to_module(
        &mut module,
        registrar_struct_name,
        route_registrations,
        middleware_expressions,
        nest_prefix,
    )?;

    Ok(module.into_token_stream().into())
}

fn add_registrar_to_module(
    module: &mut syn::ItemMod,
    registrar_struct_name: syn::Ident,
    route_registrations: Vec<TokenStream2>,
    middleware_expressions: Vec<TokenStream2>,
    nest_prefix: Option<String>,
) -> syn::Result<()> {
    let Some((_, ref mut items)) = module.content else {
        return Err(syn::Error::new(
            module.ident.span(),
            "Module must have content to apply middlewares",
        ));
    };

    let nest_prefix_expr = nest_prefix
        .as_ref()
        .map(|prefix| quote! { Some(#prefix) })
        .unwrap_or_else(|| quote! { None::<&str> });

    let registrar_struct: syn::ItemStruct = syn::parse2(quote! {
        #[allow(non_camel_case_types, missing_docs)]
        struct #registrar_struct_name;
    })?;

    let registrar_impl: syn::ItemImpl = syn::parse2(quote! {
        impl ::summer_web::handler::TypedHandlerRegistrar for #registrar_struct_name {
            fn install_route(&self, mut __router: ::summer_web::Router) -> ::summer_web::Router {
                use ::summer_web::handler::TypeRouter;

                let mut __module_router = ::summer_web::Router::new();

                #(#route_registrations)*

                __router = match #nest_prefix_expr {
                    Some(prefix) => {
                        let __catch_all_method_router = ::summer_web::axum::routing::any(|| async {
                            ::summer_web::axum::http::StatusCode::NOT_FOUND
                        });
                        __module_router = __module_router.route("/{*path}", __catch_all_method_router);

                        #(let __module_router = __module_router.layer(#middleware_expressions);)*

                        __router.nest(&prefix, __module_router)
                    },
                    None => {
                        #(let __module_router = __module_router.layer(#middleware_expressions);)*
                        __router.merge(__module_router)
                    },
                };

                __router
            }
        }
    })?;

    let submit_call: syn::ItemMacro = syn::parse2(quote! {
        ::summer_web::submit_typed_handler!(#registrar_struct_name);
    })?;

    items.extend([
        syn::Item::Struct(registrar_struct),
        syn::Item::Impl(registrar_impl),
        syn::Item::Macro(submit_call),
    ]);

    Ok(())
}

fn handle_function_middlewares(
    middleware_list: &Punctuated<Expr, Token![,]>,
    function: &syn::ItemFn,
) -> syn::Result<TokenStream> {
    let mut function_copy = function.clone();
    let route_info = extract_route_info_from_function(&function_copy)?;

    if route_info.is_empty() {
        return Err(syn::Error::new(
            function.sig.ident.span(),
            "Function must have at least one route attribute (e.g., #[get(\"/path\")])",
        ));
    }

    remove_processed_attributes(&mut function_copy.attrs);

    let func_name = &function.sig.ident;
    let registrar_struct_name =
        syn::Ident::new(&format!("{func_name}MiddlewareRegistrar"), function.sig.ident.span());

    let middleware_expressions: Vec<_> = middleware_list
        .iter()
        .rev()
        .map(|mw| quote! { #mw })
        .collect();

    let route_registrations: Vec<_> = route_info
        .iter()
        .map(|route| generate_function_route_registration(route, func_name, &middleware_expressions))
        .collect();

    Ok(quote! {
        #function_copy

        #[allow(non_camel_case_types, missing_docs)]
        struct #registrar_struct_name;

        impl ::summer_web::handler::TypedHandlerRegistrar for #registrar_struct_name {
            fn install_route(&self, mut __router: ::summer_web::Router) -> ::summer_web::Router {
                use ::summer_web::handler::TypeRouter;

                #(#route_registrations)*

                __router
            }
        }

        ::summer_web::submit_typed_handler!(#registrar_struct_name);
    }
    .into())
}
