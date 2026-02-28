use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, Expr, Token, parse::Parse};

const HTTP_METHODS: &[(&str, &str)] = &[
    ("get", "GET"),
    ("post", "POST"),
    ("put", "PUT"),
    ("delete", "DELETE"),
    ("patch", "PATCH"),
    ("head", "HEAD"),
    ("options", "OPTIONS"),
    ("trace", "TRACE"),
];

const ROUTE_ATTRS: &[&str] = &[
    "get", "post", "put", "delete", "patch", "head", "options", "trace", "route", "routes"
];

struct MiddlewareList {
    middlewares: Punctuated<Expr, Token![,]>,
}

impl Parse for MiddlewareList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let middlewares = Punctuated::<Expr, Token![,]>::parse_terminated(input)?;
        Ok(MiddlewareList { middlewares })
    }
}

fn attr_to_http_method(attr: &syn::Attribute) -> Option<&'static str> {
    HTTP_METHODS
        .iter()
        .find(|(method_name, _)| attr.path().is_ident(method_name))
        .map(|(_, http_method)| *http_method)
}

fn is_route_attr(attr: &syn::Attribute) -> bool {
    ROUTE_ATTRS.iter().any(|&route_attr| attr.path().is_ident(route_attr))
}

fn extract_and_filter_route_attrs(attrs: &mut Vec<syn::Attribute>) -> Vec<syn::Attribute> {
    let mut route_attrs = Vec::new();
    attrs.retain(|attr| {
        let is_route = is_route_attr(attr);
        let is_middleware = attr.path().is_ident("middlewares");
        
        if is_route {
            route_attrs.push(attr.clone());
            false
        } else {
            !is_middleware
        }
    });
    route_attrs
}

fn missing_args_error() -> syn::Error {
    syn::Error::new(
        Span::call_site(),
        "missing arguments for middlewares macro, expected: #[middlewares(middleware1, middleware2, ...)]",
    )
}

fn invalid_args_error(span: Span) -> syn::Error {
    syn::Error::new(
        span,
        "arguments to middlewares macro must be valid expressions, expected: #[middlewares(middleware1, middleware2, ...)]",
    )
}

pub fn middlewares(args: TokenStream, input: TokenStream) -> TokenStream {
    match middlewares_inner(args, input.clone()) {
        Ok(stream) => stream,
        Err(err) => crate::input_and_compile_error(input, err),
    }
}

fn middlewares_inner(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    if args.is_empty() {
        return Err(missing_args_error());
    }

    let middleware_list = syn::parse::<MiddlewareList>(args.clone())
        .map_err(|err| invalid_args_error(err.span()))?;

    if let Ok(function) = syn::parse::<syn::ItemFn>(input.clone()) {
        return handle_function_middlewares(&middleware_list.middlewares, &function);
    }

    handle_module_middlewares(middleware_list, input)
}

fn handle_module_middlewares(middleware_list: MiddlewareList, input: TokenStream) -> syn::Result<TokenStream> {
    let mut module = syn::parse::<syn::ItemMod>(input).map_err(|err| {
        syn::Error::new(err.span(), "#[middlewares] macro must be attached to a module")
    })?;

    let module_name = &module.ident;
    let registrar_struct_name = syn::Ident::new(
        &format!("{module_name}MiddlewareRegistrar"), 
        module.ident.span()
    );

    let nest_prefix = extract_nest_prefix(&module)?;
    let route_info = collect_and_strip_route_info(&mut module, nest_prefix.as_deref())?;
    let (route_registrations, middleware_expressions) = generate_middleware_components(
        &middleware_list.middlewares,
        &route_info,
    )?;

    add_registrar_to_module(&mut module, registrar_struct_name, route_registrations, middleware_expressions, nest_prefix)?;
    
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

    let nest_prefix_expr = nest_prefix.as_ref()
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

struct RouteInfo {
    func_name: syn::Ident,
    path: String,
    methods: Vec<String>,
    function_middlewares: Vec<syn::Expr>,
}

fn collect_and_strip_route_info(module: &mut syn::ItemMod, _nest_prefix: Option<&str>) -> syn::Result<Vec<RouteInfo>> {
    let mut route_info = Vec::new();
    
    let Some((_, items)) = &mut module.content else {
        return Ok(route_info);
    };

    for item in items {
        if let syn::Item::Fn(fun) = item {
            let function_middlewares = extract_function_middlewares(&fun.attrs)?;
            let route_attrs = extract_and_filter_route_attrs(&mut fun.attrs);
            
            for attr in route_attrs {
                if let Some(route) = process_route_attribute(&attr, &fun.sig.ident, &function_middlewares)? {
                    route_info.push(route);
                }
            }
        }
    }

    Ok(route_info)
}

fn process_route_attribute(
    attr: &syn::Attribute, 
    func_name: &syn::Ident, 
    function_middlewares: &[syn::Expr]
) -> syn::Result<Option<RouteInfo>> {
    let Some(method) = attr_to_http_method(attr) else {
        return Ok(None);
    };
    
    let Ok(path_lit) = attr.parse_args::<syn::LitStr>() else {
        return Ok(None);
    };
    
    Ok(Some(RouteInfo {
        func_name: func_name.clone(),
        path: path_lit.value(),
        methods: vec![method.to_string()],
        function_middlewares: function_middlewares.to_vec(),
    }))
}

fn extract_route_info_from_function(function: &syn::ItemFn) -> syn::Result<Vec<RouteInfo>> {
    let mut route_info = Vec::new();
    
    for attr in &function.attrs {
        if let Some(route) = process_route_attribute(attr, &function.sig.ident, &[])? {
            route_info.push(route);
        }
    }
    
    Ok(route_info)
}

fn generate_middleware_components(
    middleware_list: &Punctuated<Expr, Token![,]>,
    route_info: &[RouteInfo],
) -> syn::Result<(Vec<TokenStream2>, Vec<TokenStream2>)> {
    let route_registrations = route_info
        .iter()
        .map(generate_route_registration)
        .collect();

    let middleware_expressions = middleware_list
        .iter()
        .rev()
        .map(|middleware| quote! { #middleware })
        .collect();

    Ok((route_registrations, middleware_expressions))
}

fn generate_route_registration(route: &RouteInfo) -> TokenStream2 {
    let func_name = &route.func_name;
    let path = &route.path;
    let methods = generate_method_filters(&route.methods);
    
    if route.function_middlewares.is_empty() {
        quote! { 
            let __method_router = ::summer_web::MethodRouter::new();
            #(let __method_router = ::summer_web::MethodRouter::on(__method_router, #methods, #func_name);)*
            __module_router = ::summer_web::Router::route(__module_router, #path, __method_router);
        }
    } else {
        let function_middleware_layers = route.function_middlewares
            .iter()
            .rev()
            .map(|middleware| quote! { #middleware })
            .collect::<Vec<_>>();
        
        quote! { 
            let mut __function_router = ::summer_web::Router::new();
            let __method_router = ::summer_web::MethodRouter::new();
            #(let __method_router = ::summer_web::MethodRouter::on(__method_router, #methods, #func_name);)*
            __function_router = ::summer_web::Router::route(__function_router, #path, __method_router);
            
            #(let __function_router = __function_router.layer(#function_middleware_layers);)*
            
            __module_router = __module_router.merge(__function_router);
        }
    }
}

fn generate_method_filters(methods: &[String]) -> Vec<TokenStream2> {
    methods
        .iter()
        .map(|method_str| {
            let method_ident = syn::Ident::new(method_str, Span::call_site());
            quote! { ::summer_web::MethodFilter::#method_ident }
        })
        .collect()
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
    let registrar_struct_name = syn::Ident::new(
        &format!("{func_name}MiddlewareRegistrar"),
        function.sig.ident.span()
    );
    
    let middleware_expressions = middleware_list
        .iter()
        .rev()
        .map(|middleware| quote! { #middleware })
        .collect::<Vec<_>>();
    
    let route_registrations = route_info
        .iter()
        .map(|route| generate_function_route_registration(route, func_name, &middleware_expressions))
        .collect::<Vec<_>>();
    
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
    }.into())
}

fn remove_processed_attributes(attrs: &mut Vec<syn::Attribute>) {
    attrs.retain(|attr| {
        !attr.path().is_ident("middlewares") && !is_route_attr(attr)
    });
}

fn generate_function_route_registration(
    route: &RouteInfo, 
    func_name: &syn::Ident, 
    middleware_expressions: &[TokenStream2]
) -> TokenStream2 {
    let path = &route.path;
    let methods = generate_method_filters(&route.methods);
    
    quote! { 
        let mut __method_router = ::summer_web::MethodRouter::new();
        #(let __method_router = ::summer_web::MethodRouter::on(__method_router, #methods, #func_name);)*
        
        #(let __method_router = __method_router.layer(#middleware_expressions);)*
        
        __router = ::summer_web::Router::route(__router, #path, __method_router);
    }
}
