use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ExprPath, Ident, ItemFn, LitStr, ReturnType, Token, Type};

struct CircuitBreakerArgs {
    name: LitStr,
    record_failure: Option<ExprPath>,
}

impl Parse for CircuitBreakerArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut record_failure = None;
        while !input.is_empty() {
            let key = input.parse::<Ident>()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "name" => {
                    let value = input.parse::<LitStr>()?;
                    if name.replace(value).is_some() {
                        return Err(syn::Error::new_spanned(key, "duplicate `name` parameter"));
                    }
                }
                "record_failure" => {
                    let value = input.parse::<ExprPath>()?;
                    if record_failure.replace(value).is_some() {
                        return Err(syn::Error::new_spanned(
                            key,
                            "duplicate `record_failure` parameter",
                        ));
                    }
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        "unknown circuit breaker parameter; expected `name` or `record_failure`",
                    ));
                }
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        let name = name.ok_or_else(|| input.error("missing required `name` parameter"))?;
        if name.value().is_empty() {
            return Err(syn::Error::new_spanned(
                name,
                "circuit breaker name cannot be empty",
            ));
        }
        Ok(Self {
            name,
            record_failure,
        })
    }
}

pub fn circuit_breaker(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as CircuitBreakerArgs);
    let function = syn::parse_macro_input!(item as ItemFn);
    expand(args, function)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(args: CircuitBreakerArgs, function: ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    if function.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(
            function.sig.fn_token,
            "#[circuit_breaker] only supports async functions",
        ));
    }
    if function.sig.unsafety.is_some() {
        return Err(syn::Error::new_spanned(
            function.sig.unsafety,
            "#[circuit_breaker] does not support unsafe functions",
        ));
    }
    if !returns_result(&function.sig.output) {
        return Err(syn::Error::new_spanned(
            &function.sig.output,
            "#[circuit_breaker] functions must return Result<T, E>",
        ));
    }

    let attrs = &function.attrs;
    let vis = &function.vis;
    let sig = &function.sig;
    let block = &function.block;
    let name = args.name;
    let record_failure = args
        .record_failure
        .map(|path| quote! { #path })
        .unwrap_or_else(|| quote! { |_: &_| true });

    Ok(quote! {
        #(#attrs)*
        #vis #sig {
            use ::summer::plugin::ComponentRegistry as _;

            let __summer_circuit_breaker_registry = ::summer::App::global()
                .get_expect_component::<::summer_resilience::CircuitBreakerRegistry>();
            let __summer_circuit_breaker = __summer_circuit_breaker_registry.get(#name)
                .unwrap_or_else(|| panic!("circuit breaker `{}` is not configured", #name));

            match ::summer_resilience::circuit_breaker::execute(
                __summer_circuit_breaker,
                || async move #block,
                #record_failure,
            )
            .await
            {
                ::core::result::Result::Ok(value) => ::core::result::Result::Ok(value),
                ::core::result::Result::Err(
                    ::summer_resilience::CircuitBreakerError::Operation(error),
                ) => ::core::result::Result::Err(error),
                ::core::result::Result::Err(
                    ::summer_resilience::CircuitBreakerError::CallNotPermitted(error),
                ) => ::core::result::Result::Err(::core::convert::From::from(error)),
            }
        }
    })
}

fn returns_result(output: &ReturnType) -> bool {
    let ReturnType::Type(_, ty) = output else {
        return false;
    };
    let Type::Path(type_path) = ty.as_ref() else {
        return false;
    };
    type_path
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "Result")
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn args() -> CircuitBreakerArgs {
        CircuitBreakerArgs {
            name: LitStr::new("test", proc_macro2::Span::call_site()),
            record_failure: None,
        }
    }

    #[test]
    fn rejects_synchronous_functions() {
        let function: ItemFn = parse_quote! {
            fn operation() -> Result<(), ()> { Ok(()) }
        };
        assert!(expand(args(), function)
            .unwrap_err()
            .to_string()
            .contains("async"));
    }

    #[test]
    fn rejects_non_result_functions() {
        let function: ItemFn = parse_quote! {
            async fn operation() -> usize { 1 }
        };
        assert!(expand(args(), function)
            .unwrap_err()
            .to_string()
            .contains("Result"));
    }
}
