use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Expr, Lit, Type};

pub(crate) fn expand_validator_context(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let ident = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let mut context_ty: Option<Type> = None;
    let mut mutable_context = false;

    for attr in &input.attrs {
        if !attr.path().is_ident("validate") {
            continue;
        }

        let nested = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        )?;

        for meta in &nested {
            match meta {
                syn::Meta::NameValue(nv) if nv.path.is_ident("context") => {
                    context_ty = Some(parse_context_type(&nv.value)?);
                }
                syn::Meta::Path(path) if path.is_ident("mutable") => {
                    mutable_context = true;
                }
                _ => {}
            }
        }
    }

    if mutable_context {
        return Err(syn::Error::new_spanned(
            &ident,
            "summer-web ValidatorContext does not support mutable validator contexts yet",
        ));
    }

    let context_ty = context_ty.ok_or_else(|| {
        syn::Error::new_spanned(
            &ident,
            "ValidatorContext requires #[validate(context = ...)] on the type",
        )
    })?;

    Ok(quote! {
        impl #impl_generics ::summer_web::ValidatorContextType for #ident #ty_generics #where_clause {
            type Context = #context_ty;
        }
    })
}

fn parse_context_type(expr: &Expr) -> syn::Result<Type> {
    match expr {
        Expr::Path(expr_path) => Ok(Type::Path(syn::TypePath {
            qself: None,
            path: expr_path.path.clone(),
        })),
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            Lit::Str(lit) => lit.parse::<Type>(),
            _ => Err(syn::Error::new_spanned(
                expr,
                "validate context must be a type path or string literal type",
            )),
        },
        _ => Err(syn::Error::new_spanned(
            expr,
            "validate context must be a type path or string literal type",
        )),
    }
}
