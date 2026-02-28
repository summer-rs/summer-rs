//! Utility functions for the component macro

use syn::{GenericArgument, PathArguments, Type};

/// Extract the generic type T from a wrapper type like Config<T> or Component<T>
pub fn extract_generic_type(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if let PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                    return Some(inner_ty.clone());
                }
            }
        }
    }
    None
}

/// Extract the return type from function signature
pub fn extract_return_type(output: &syn::ReturnType) -> Option<Type> {
    match output {
        syn::ReturnType::Default => None,
        syn::ReturnType::Type(_, ty) => Some((**ty).clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse2;

    #[test]
    fn test_extract_generic_type() {
        let ty: Type = parse2(quote! { Config<DbConfig> }).unwrap();
        let inner = extract_generic_type(&ty).unwrap();
        assert_eq!(quote!(#inner).to_string(), "DbConfig");
    }

    #[test]
    fn test_extract_generic_type_from_result() {
        let ty: Type = parse2(quote! { Result<MyComponent, Error> }).unwrap();
        let ok_ty = extract_generic_type(&ty).unwrap();
        assert_eq!(quote!(#ok_ty).to_string(), "MyComponent");
    }

    #[test]
    fn test_extract_return_type() {
        let output: syn::ReturnType = parse2(quote! { -> MyComponent }).unwrap();
        let ty = extract_return_type(&output).unwrap();
        assert_eq!(quote!(#ty).to_string(), "MyComponent");
    }
}
