use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{ExprPath, FnArg, Ident, ItemFn, LitStr, Pat, ReturnType, Token, Type};

struct RetryArgs {
    name: LitStr,
    retry_if: Option<ExprPath>,
}

impl Parse for RetryArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut retry_if = None;

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
                "retry_if" => {
                    let value = input.parse::<ExprPath>()?;
                    if retry_if.replace(value).is_some() {
                        return Err(syn::Error::new_spanned(
                            key,
                            "duplicate `retry_if` parameter",
                        ));
                    }
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        "unknown retry parameter; expected `name` or `retry_if`",
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
                "retry policy name cannot be empty",
            ));
        }

        Ok(Self { name, retry_if })
    }
}

pub fn retry(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as RetryArgs);
    let function = syn::parse_macro_input!(item as ItemFn);

    expand(args, function)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(args: RetryArgs, function: ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    if function.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(
            function.sig.fn_token,
            "#[retry] only supports async functions",
        ));
    }
    if function.sig.unsafety.is_some() {
        return Err(syn::Error::new_spanned(
            function.sig.unsafety,
            "#[retry] does not support unsafe functions",
        ));
    }
    if !returns_result(&function.sig.output) {
        return Err(syn::Error::new_spanned(
            &function.sig.output,
            "#[retry] functions must return Result<T, E>",
        ));
    }

    let mut clones = Vec::new();
    for input in &function.sig.inputs {
        match input {
            FnArg::Receiver(receiver) => {
                if receiver.reference.is_none() || receiver.mutability.is_some() {
                    return Err(syn::Error::new_spanned(
                        receiver,
                        "#[retry] methods require an immutable `&self` receiver",
                    ));
                }
            }
            FnArg::Typed(argument) => {
                if matches!(argument.ty.as_ref(), Type::Reference(reference) if reference.mutability.is_some())
                {
                    return Err(syn::Error::new_spanned(
                        &argument.ty,
                        "#[retry] does not support mutable reference parameters",
                    ));
                }
                let Pat::Ident(pattern) = argument.pat.as_ref() else {
                    return Err(syn::Error::new_spanned(
                        &argument.pat,
                        "#[retry] requires identifier parameters; destructure values inside the function body",
                    ));
                };
                if pattern.by_ref.is_some() || pattern.subpat.is_some() {
                    return Err(syn::Error::new_spanned(
                        pattern,
                        "#[retry] does not support `ref` or subpattern parameters",
                    ));
                }
                let ident = &pattern.ident;
                let mutability = &pattern.mutability;
                clones.push(quote! {
                    let #mutability #ident = ::core::clone::Clone::clone(&#ident);
                });
            }
        }
    }

    let attrs = &function.attrs;
    let vis = &function.vis;
    let sig = &function.sig;
    let block = &function.block;
    let name = args.name;
    let retry_if = args
        .retry_if
        .map(|path| quote! { #path })
        .unwrap_or_else(|| quote! { |_: &_| true });

    Ok(quote! {
        #(#attrs)*
        #vis #sig {
            use ::summer::plugin::ComponentRegistry as _;

            let __summer_retry_registry = ::summer::App::global()
                .get_expect_component::<::summer_resilience::RetryRegistry>();
            let __summer_retry_policy = __summer_retry_registry.get(#name)
                .unwrap_or_else(|| panic!("retry policy `{}` is not configured", #name));

            ::summer_resilience::retry::execute(
                #name,
                __summer_retry_policy,
                || {
                    #(#clones)*
                    async move #block
                },
                #retry_if,
            )
            .await
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

    fn args() -> RetryArgs {
        RetryArgs {
            name: LitStr::new("test", proc_macro2::Span::call_site()),
            retry_if: None,
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

    #[test]
    fn rejects_destructured_parameters() {
        let function: ItemFn = parse_quote! {
            async fn operation((value, _): (usize, usize)) -> Result<usize, ()> { Ok(value) }
        };

        assert!(expand(args(), function)
            .unwrap_err()
            .to_string()
            .contains("identifier parameters"));
    }
}
