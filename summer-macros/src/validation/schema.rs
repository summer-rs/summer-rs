//! `#[derive(GardeSchema)]` and `#[derive(ValidatorSchema)]` — generate
//! `schemars::JsonSchema` while filtering validation attributes that can
//! conflict with schema derivation.

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Attribute, DeriveInput, Field, Fields, Generics};

fn filter_attrs(attrs: &[Attribute], validation_attr: &str) -> Vec<Attribute> {
    attrs.iter()
        .filter(|attr| {
            let path = attr.path();
            !path.is_ident(validation_attr) && !path.is_ident("derive")
        })
        .cloned()
        .collect()
}

fn filtered_named_fields(
    fields: &Fields,
    validation_attr: &str,
) -> syn::Result<Vec<Field>> {
    let named = match fields {
        Fields::Named(named) => named,
        _ => {
            return Err(syn::Error::new_spanned(
                fields,
                "derive macro requires named fields",
            ))
        }
    };

    Ok(named
        .named
        .iter()
        .cloned()
        .map(|mut field| {
            field.attrs = filter_attrs(&field.attrs, validation_attr);
            field
        })
        .collect())
}

fn generate_helper_struct(
    helper_ident: &Ident,
    struct_attrs: &[Attribute],
    helper_fields: &[Field],
    generics: &Generics,
) -> TokenStream {
    quote! {
        #[allow(non_camel_case_types)]
        #[derive(::schemars::JsonSchema)]
        #(#struct_attrs)*
        struct #helper_ident #generics {
            #(#helper_fields),*
        }
    }
}

fn expand_schema_derive(
    input: DeriveInput,
    macro_name: &str,
    validation_attr: &str,
) -> syn::Result<TokenStream> {
    let struct_ident = input.ident;
    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let data = match input.data {
        syn::Data::Struct(data) => data,
        _ => {
            return Err(syn::Error::new_spanned(
                &struct_ident,
                format!("{macro_name} can only be derived for structs"),
            ))
        }
    };

    let struct_attrs = filter_attrs(&input.attrs, validation_attr);
    let helper_fields = filtered_named_fields(&data.fields, validation_attr)?;
    let helper_ident = format_ident!("__SummerSchemaBase_{}", struct_ident);
    let helper_struct =
        generate_helper_struct(&helper_ident, &struct_attrs, &helper_fields, &generics);

    let helper_name = helper_ident.to_string();
    let original_name = struct_ident.to_string();

    Ok(quote! {
        #helper_struct

        impl #impl_generics ::schemars::JsonSchema for #struct_ident #ty_generics #where_clause {
            fn inline_schema() -> bool {
                <#helper_ident #ty_generics as ::schemars::JsonSchema>::inline_schema()
            }

            fn schema_name() -> ::std::borrow::Cow<'static, str> {
                let helper = <#helper_ident #ty_generics as ::schemars::JsonSchema>::schema_name()
                    .into_owned();
                helper.replace(#helper_name, #original_name).into()
            }

            fn schema_id() -> ::std::borrow::Cow<'static, str> {
                let helper = <#helper_ident #ty_generics as ::schemars::JsonSchema>::schema_id()
                    .into_owned();
                helper.replace(#helper_name, #original_name).into()
            }

            fn json_schema(generator: &mut ::schemars::SchemaGenerator) -> ::schemars::Schema {
                <#helper_ident #ty_generics as ::schemars::JsonSchema>::json_schema(generator)
            }
        }
    })
}

#[cfg(feature = "garde")]
pub(crate) fn expand_garde(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    expand_schema_derive(input, "GardeSchema", "garde")
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[cfg(feature = "validator")]
pub(crate) fn expand_validator(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    expand_schema_derive(input, "ValidatorSchema", "validate")
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
