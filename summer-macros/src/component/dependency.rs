//! Dependency analysis for the `#[component]` macro

use syn::{Attribute, FnArg, ItemFn, LitStr, Meta, Type};

use crate::component::utils::extract_generic_type;

/// Information about a component dependency
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    /// The component type (T in Component<T>)
    pub component_type: Type,
    /// Explicitly specified Plugin name via #[inject("PluginName")]
    pub explicit_plugin_name: Option<String>,
    /// Inferred Plugin name based on type
    pub inferred_plugin_name: String,
}

/// Analyze dependencies from function parameters
pub fn analyze_dependencies(func: &ItemFn) -> Vec<DependencyInfo> {
    let mut dependencies = Vec::new();

    for param in &func.sig.inputs {
        if let FnArg::Typed(pat_type) = param {
            let ty = &*pat_type.ty;

            // Check if it's Component<T>
            if is_component_type(ty) {
                if let Some(component_type) = extract_generic_type(ty) {
                    // Check for #[inject("PluginName")] attribute
                    let explicit_name = extract_inject_attr(&pat_type.attrs);

                    // Infer Plugin name from type
                    let inferred_name = infer_plugin_name(&component_type);

                    dependencies.push(DependencyInfo {
                        component_type,
                        explicit_plugin_name: explicit_name,
                        inferred_plugin_name: inferred_name,
                    });
                }
            }
        }
    }

    dependencies
}

/// Check if a type is Component<T>
fn is_component_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Component";
        }
    }
    false
}

/// Extract #[inject("PluginName")] attribute
fn extract_inject_attr(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("inject") {
            if let Meta::List(meta_list) = &attr.meta {
                // Parse the string literal inside inject("...")
                if let Ok(lit) = syn::parse2::<LitStr>(meta_list.tokens.clone()) {
                    return Some(lit.value());
                }
            }
        }
    }
    None
}

/// Infer Plugin name from component type
/// Format: __Create{TypeName}Plugin
fn infer_plugin_name(component_type: &Type) -> String {
    let type_name = extract_type_name(component_type);
    format!("__Create{}Plugin", type_name)
}

/// Extract the type name from a Type
fn extract_type_name(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                segment.ident.to_string()
            } else {
                "Unknown".to_string()
            }
        }
        _ => "Unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse2;

    #[test]
    fn test_analyze_dependencies_no_deps() {
        let input = quote! {
            fn create_component(Config(config): Config<MyConfig>) -> MyComponent {
                MyComponent::new(config)
            }
        };
        let func = parse2::<ItemFn>(input).unwrap();
        let deps = analyze_dependencies(&func);
        assert_eq!(deps.len(), 0);
    }

    #[test]
    fn test_analyze_dependencies_with_component() {
        let input = quote! {
            fn create_service(
                Component(db): Component<DbConnection>,
            ) -> MyService {
                MyService::new(db)
            }
        };
        let func = parse2::<ItemFn>(input).unwrap();
        let deps = analyze_dependencies(&func);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].inferred_plugin_name, "__CreateDbConnectionPlugin");
    }

    #[test]
    fn test_infer_plugin_name() {
        let ty: Type = parse2(quote! { UserRepository }).unwrap();
        let name = infer_plugin_name(&ty);
        assert_eq!(name, "__CreateUserRepositoryPlugin");
    }

    #[test]
    fn test_extract_type_name() {
        let ty: Type = parse2(quote! { DbConnection }).unwrap();
        let name = extract_type_name(&ty);
        assert_eq!(name, "DbConnection");
    }
}
