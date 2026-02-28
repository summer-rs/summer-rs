//! Function signature validation for the `#[component]` macro

use syn::{FnArg, ItemFn, Result, ReturnType, Type};

/// Validate the function signature for `#[component]` macro
pub fn validate_function_signature(func: &ItemFn) -> Result<()> {
    // Check if it's a method (has self parameter)
    if func.sig.receiver().is_some() {
        return Err(syn::Error::new_spanned(
            &func.sig,
            "#[component] can only be applied to standalone functions, not methods",
        ));
    }

    // Validate return type
    validate_return_type(&func.sig.output)?;

    // Validate parameters
    for param in &func.sig.inputs {
        validate_parameter(param)?;
    }

    Ok(())
}

/// Validate that the return type implements Clone
fn validate_return_type(output: &ReturnType) -> Result<()> {
    match output {
        ReturnType::Default => {
            Err(syn::Error::new_spanned(
                output,
                "#[component] function must return a value (not unit type)",
            ))
        }
        ReturnType::Type(_, ty) => {
            // We can't check Clone trait at compile time in proc macro,
            // but we can check if it's a valid type
            // The actual Clone check will happen at the usage site
            
            // Check if it's Result<T, E> - extract T
            if is_result_type(ty) {
                // Result type is acceptable
                return Ok(());
            }
            
            // Other types are acceptable, Clone will be checked at usage
            Ok(())
        }
    }
}

/// Check if a type is Result<T, E>
pub fn is_result_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Result";
        }
    }
    false
}

/// Validate function parameter
fn validate_parameter(param: &FnArg) -> Result<()> {
    match param {
        FnArg::Receiver(_) => {
            // This should be caught by the receiver check above
            Err(syn::Error::new_spanned(
                param,
                "#[component] cannot be used on methods",
            ))
        }
        FnArg::Typed(pat_type) => {
            let ty = &*pat_type.ty;
            
            // Check if it's Config<T> or Component<T>
            if !is_config_type(ty) && !is_component_type(ty) {
                return Err(syn::Error::new_spanned(
                    ty,
                    "Parameter must be Config<T> or Component<T>",
                ));
            }
            
            Ok(())
        }
    }
}

/// Check if a type is Config<T>
pub fn is_config_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Config";
        }
    }
    false
}

/// Check if a type is Component<T>
pub fn is_component_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Component";
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse2;

    #[test]
    fn test_validate_standalone_function() {
        let input = quote! {
            fn create_component(Config(config): Config<MyConfig>) -> MyComponent {
                MyComponent::new(config)
            }
        };
        let func = parse2::<ItemFn>(input).unwrap();
        assert!(validate_function_signature(&func).is_ok());
    }

    #[test]
    fn test_reject_method() {
        let input = quote! {
            fn create_component(&self) -> MyComponent {
                MyComponent
            }
        };
        let func = parse2::<ItemFn>(input).unwrap();
        assert!(validate_function_signature(&func).is_err());
    }

    #[test]
    fn test_reject_invalid_parameter() {
        let input = quote! {
            fn create_component(invalid: String) -> MyComponent {
                MyComponent
            }
        };
        let func = parse2::<ItemFn>(input).unwrap();
        assert!(validate_function_signature(&func).is_err());
    }

    #[test]
    fn test_accept_result_return_type() {
        let input = quote! {
            fn create_component() -> Result<MyComponent, Error> {
                Ok(MyComponent)
            }
        };
        let func = parse2::<ItemFn>(input).unwrap();
        assert!(validate_function_signature(&func).is_ok());
    }
}
