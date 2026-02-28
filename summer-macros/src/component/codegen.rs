//! Code generation for the `#[component]` macro

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn};

use super::{attrs::ComponentAttrs, dependency::DependencyInfo, utils};

/// Generate Plugin implementation
pub fn generate_plugin_impl(
    attrs: &ComponentAttrs,
    func: &ItemFn,
    dependencies: &[DependencyInfo],
) -> TokenStream {
    let func_name = &func.sig.ident;
    
    // Extract return type
    let return_type = match utils::extract_return_type(&func.sig.output) {
        Some(ty) => ty,
        None => {
            return quote! {
                compile_error!("#[component] function must return a value");
            }
        }
    };

    // Generate Plugin struct name
    let plugin_struct_name = generate_plugin_struct_name(attrs, &return_type);

    // Generate Plugin logical name (for name() method)
    let plugin_logical_name = attrs
        .name
        .clone()
        .unwrap_or_else(|| format!("__Create{}Plugin", extract_type_name(&return_type)));

    // Generate dependency names as string literals
    let dependency_names: Vec<_> = dependencies
        .iter()
        .map(|dep| {
            dep.explicit_plugin_name
                .clone()
                .unwrap_or_else(|| dep.inferred_plugin_name.clone())
        })
        .collect();

    // Generate parameter extractions
    let param_extractions = generate_param_extractions(func, dependencies);

    // Generate function call
    let param_names = extract_param_names(func);
    let func_call = quote! { #func_name(#(#param_names),*) };

    // Check if async
    let is_async = func.sig.asyncness.is_some();
    let await_token = if is_async {
        quote! { .await }
    } else {
        quote! {}
    };

    // Check if Result type
    let is_result = is_result_type(&return_type);
    let result_handling = if is_result {
        quote! {
            let component = #func_call #await_token
                .expect(&format!("Failed to create component in {}", #plugin_logical_name));
        }
    } else {
        quote! {
            let component = #func_call #await_token;
        }
    };

    quote! {
        struct #plugin_struct_name;

        #[::summer::async_trait]
        impl ::summer::plugin::Plugin for #plugin_struct_name {
            async fn build(&self, app: &mut ::summer::app::AppBuilder) {
                use ::summer::config::ConfigRegistry;
                use ::summer::plugin::{ComponentRegistry, MutableComponentRegistry};
                
                #(#param_extractions)*

                #result_handling

                app.add_component(component);
            }

            fn name(&self) -> &str {
                #plugin_logical_name
            }

            fn dependencies(&self) -> Vec<&str> {
                vec![#(#dependency_names),*]
            }
        }
    }
}

/// Generate inventory registration
pub fn generate_inventory_submit(attrs: &ComponentAttrs, func: &ItemFn) -> TokenStream {
    let return_type = match utils::extract_return_type(&func.sig.output) {
        Some(ty) => ty,
        None => return quote! {},
    };

    let plugin_struct_name = generate_plugin_struct_name(attrs, &return_type);

    quote! {
        ::summer::submit_component_plugin!(#plugin_struct_name);
    }
}

/// Generate Plugin struct name
fn generate_plugin_struct_name(attrs: &ComponentAttrs, return_type: &syn::Type) -> syn::Ident {
    if let Some(name) = &attrs.name {
        // Use custom name with __ prefix
        format_ident!("__{}", name)
    } else {
        // Auto-generate name
        let type_name = extract_type_name(return_type);
        format_ident!("__Create{}Plugin", type_name)
    }
}

/// Extract type name from Type
fn extract_type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                segment.ident.to_string()
            } else {
                "Unknown".to_string()
            }
        }
        _ => "Unknown".to_string(),
    }
}

/// Check if type is Result<T, E>
fn is_result_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Result";
        }
    }
    false
}

/// Generate parameter extraction code
fn generate_param_extractions(
    func: &ItemFn,
    dependencies: &[DependencyInfo],
) -> Vec<TokenStream> {
    let mut extractions = Vec::new();
    let mut dep_index = 0;

    for param in &func.sig.inputs {
        if let FnArg::Typed(pat_type) = param {
            let pat = &pat_type.pat;
            let ty = &*pat_type.ty;

            if is_config_type(ty) {
                // Extract Config<T>
                if let Some(config_type) = utils::extract_generic_type(ty) {
                    extractions.push(quote! {
                        let #pat = {
                            let config = app.get_config::<#config_type>()
                                .expect(&format!("Config {} not found", stringify!(#config_type)));
                            ::summer::extractor::Config(config)
                        };
                    });
                }
            } else if is_component_type(ty) {
                // Extract Component<T>
                if dep_index < dependencies.len() {
                    let component_type = &dependencies[dep_index].component_type;
                    extractions.push(quote! {
                        let #pat = {
                            let component = app.get_component::<#component_type>()
                                .expect(&format!("Component {} not found", stringify!(#component_type)));
                            ::summer::extractor::Component(component)
                        };
                    });
                    dep_index += 1;
                }
            }
        }
    }

    extractions
}

/// Extract parameter names from function
fn extract_param_names(func: &ItemFn) -> Vec<&syn::Pat> {
    func.sig
        .inputs
        .iter()
        .filter_map(|param| {
            if let FnArg::Typed(pat_type) = param {
                Some(&*pat_type.pat)
            } else {
                None
            }
        })
        .collect()
}

/// Check if type is Config<T>
fn is_config_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Config";
        }
    }
    false
}

/// Check if type is Component<T>
fn is_component_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Component";
        }
    }
    false
}
