//! Attribute parsing for the `#[component]` macro

use proc_macro2::TokenStream;
use syn::{parse::Parse, parse2, LitStr, Result};

/// Attributes for the `#[component]` macro
#[derive(Debug, Default)]
pub struct ComponentAttrs {
    /// Custom Plugin name (optional)
    pub name: Option<String>,
}

impl Parse for ComponentAttrs {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut attrs = ComponentAttrs::default();

        if input.is_empty() {
            return Ok(attrs);
        }

        // Parse `name = "PluginName"`
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Ident) {
            let ident: syn::Ident = input.parse()?;
            if ident == "name" {
                input.parse::<syn::Token![=]>()?;
                let lit: LitStr = input.parse()?;
                attrs.name = Some(lit.value());
            } else {
                return Err(syn::Error::new_spanned(
                    ident,
                    "unsupported attribute, only `name` is supported",
                ));
            }
        }

        Ok(attrs)
    }
}

/// Parse component attributes from token stream
pub fn parse_component_attrs(attr: TokenStream) -> Result<ComponentAttrs> {
    if attr.is_empty() {
        Ok(ComponentAttrs::default())
    } else {
        parse2::<ComponentAttrs>(attr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_parse_empty_attrs() {
        let input = quote! {};
        let attrs = parse_component_attrs(input).unwrap();
        assert!(attrs.name.is_none());
    }

    #[test]
    fn test_parse_name_attr() {
        let input = quote! { name = "MyPlugin" };
        let attrs = parse_component_attrs(input).unwrap();
        assert_eq!(attrs.name, Some("MyPlugin".to_string()));
    }

    #[test]
    fn test_parse_invalid_attr() {
        let input = quote! { invalid = "value" };
        let result = parse_component_attrs(input);
        assert!(result.is_err());
    }
}
