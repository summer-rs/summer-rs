use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Attribute};

pub(crate) fn expand_derive(input: DeriveInput) -> syn::Result<TokenStream> {
    let ident = &input.ident;

    let data_enum = match &input.data {
        Data::Enum(data_enum) => data_enum,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "ProblemDetails can only be derived for enums",
            ))
        }
    };

    let mut match_arms = Vec::new();
    let mut variant_info_arms = Vec::new();
    let mut problem_details_arms = Vec::new();

    for variant in &data_enum.variants {
        let variant_ident = &variant.ident;
        let variant_name = variant_ident.to_string();

        let status_code_lit = get_status_code_literal(&variant.attrs)?;
        let status_code = quote! {
            ::summer_web::axum::http::StatusCode::from_u16(#status_code_lit).unwrap()
        };

        let pattern = match &variant.fields {
            Fields::Unit => quote! { #ident::#variant_ident },
            Fields::Unnamed(_) => quote! { #ident::#variant_ident(..) },
            Fields::Named(_) => quote! { #ident::#variant_ident { .. } },
        };

        match_arms.push(quote! {
            #pattern => #status_code
        });

        let is_transparent = has_transparent_attribute(&variant.attrs);

        let (description_str, inner_type_opt) = match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field = fields.unnamed.first().unwrap();
                let inner_type = &field.ty;

                if is_transparent {
                    let inner_type_str = quote!(#inner_type).to_string().replace(' ', "");
                    (format!("{} (wraps {})", variant_name, inner_type_str), Some(inner_type.clone()))
                } else {
                    (format!("{} error", variant_name), Some(inner_type.clone()))
                }
            }
            _ => (format!("{} error{}", variant_name, if is_transparent { " (transparent)" } else { "" }), None)
        };

        let schema_gen = if let Some(inner_type) = inner_type_opt {
            if is_transparent {
                quote! {
                    {
                        let schema_opt: Option<::schemars::Schema> = None;
                        (
                            #status_code_lit,
                            #description_str.to_string(),
                            schema_opt
                        )
                    }
                }
            } else {
                quote! {
                    {
                        let mut gen = ::schemars::SchemaGenerator::default();
                        let schema_opt: Option<::schemars::Schema> = 
                            Some(<#inner_type as ::schemars::JsonSchema>::json_schema(&mut gen));
                        (
                            #status_code_lit,
                            #description_str.to_string(),
                            schema_opt
                        )
                    }
                }
            }
        } else {
            quote! {
                (#status_code_lit, #description_str.to_string(), None)
            }
        };

        variant_info_arms.push(quote! {
            #variant_name => Some(#schema_gen)
        });

        // Generate Problem Details mapping with support for custom attributes
        let problem_details_expr = generate_problem_details_for_variant(
            ident,
            &variant.ident,
            &variant.fields,
            status_code_lit, 
            &variant_name, 
            &variant.attrs
        )?;
        
        let pattern = match &variant.fields {
            Fields::Unit => quote! { #ident::#variant_ident },
            Fields::Unnamed(_) => quote! { #ident::#variant_ident(ref inner) },
            Fields::Named(_) => quote! { #ident::#variant_ident { .. } },
        };
        
        problem_details_arms.push(quote! {
            #pattern => #problem_details_expr
        });
    }

    let mod_name = quote::format_ident!("__problem_details_impl_{}", ident.to_string().to_lowercase());
    
    let output = quote! {
        // OpenAPI integration implementations
        impl ::summer_web::aide::OperationOutput for #ident {
            type Inner = Self;

            fn operation_response(
                _ctx: &mut ::summer_web::aide::generate::GenContext,
                _operation: &mut ::summer_web::aide::openapi::Operation,
            ) -> Option<::summer_web::aide::openapi::Response> {
                None
            }

            fn inferred_responses(
                _ctx: &mut ::summer_web::aide::generate::GenContext,
                _operation: &mut ::summer_web::aide::openapi::Operation,
            ) -> Vec<(Option<::summer_web::aide::openapi::StatusCode>, ::summer_web::aide::openapi::Response)> {
                vec![]
            }
        }

        impl ::summer_web::openapi::ProblemDetailsVariantInfo for #ident {
            fn get_variant_info(variant_name: &str) -> Option<(u16, String, Option<::schemars::Schema>)> {
                Self::__get_variant_info(variant_name)
            }
        }

        #[doc(hidden)]
        mod #mod_name {
            use super::*;
            
            pub fn get_variant_info(variant_name: &str) -> Option<(u16, String, Option<::schemars::Schema>)> {
                match variant_name {
                    #(#variant_info_arms,)*
                    _ => None
                }
            }
        }

        impl #ident {
            #[doc(hidden)]
            pub fn __get_variant_info(variant_name: &str) -> Option<(u16, String, Option<::schemars::Schema>)> {
                #mod_name::get_variant_info(variant_name)
            }
        }

        impl From<#ident> for ::summer_web::problem_details::ProblemDetails {
            fn from(error: #ident) -> Self {
                match error {
                    #(#problem_details_arms),*
                }
            }
        }

        impl ::summer_web::axum::response::IntoResponse for #ident {
            fn into_response(self) -> ::summer_web::axum::response::Response {
                let problem_details: ::summer_web::problem_details::ProblemDetails = self.into();
                problem_details.into_response()
            }
        }
    };

    Ok(output)
}

fn get_status_code_literal(attrs: &[syn::Attribute]) -> syn::Result<u16> {
    for attr in attrs {
        if attr.path().is_ident("status_code") {
            // Parse the attribute value
            let status_code: syn::LitInt = attr.parse_args()?;
            return status_code.base10_parse::<u16>();
        }
    }

    Err(syn::Error::new_spanned(
        attrs.first(),
        "Missing #[status_code(...)] attribute. Each variant must have a status_code attribute, e.g., #[status_code(404)]",
    ))
}

fn has_transparent_attribute(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("error") {
            if let Ok(meta) = attr.parse_args::<syn::Ident>() {
                if meta == "transparent" {
                    return true;
                }
            }
        }
    }
    false
}

fn generate_problem_details_for_variant(
    enum_ident: &syn::Ident,
    variant_ident: &syn::Ident,
    variant_fields: &Fields,
    status_code: u16, 
    variant_name: &str, 
    attrs: &[Attribute]
) -> syn::Result<TokenStream> {
    // 解析自定义属性
    let problem_type = get_problem_type_from_attrs(attrs)?;
    let title = get_title_from_attrs(attrs)?;
    let detail = get_detail_from_attrs(attrs)?;
    let instance = get_instance_from_attrs(attrs)?;
    let error_format = get_error_format_from_attrs(attrs)?;
    
    // 如果有自定义的 problem_type，使用它；否则根据状态码自动生成
    let problem_details_expr = if let Some(custom_type) = problem_type {
        let title_expr = if let Some(title_val) = title {
            quote! { #title_val.to_string() }
        } else if let Some(error_fmt) = &error_format {
            // 如果有格式化的 error 属性，使用格式化后的字符串作为 title
            generate_format_expr(enum_ident, variant_ident, variant_fields, error_fmt)?
        } else {
            let default_title = format!("{} Error", variant_name);
            quote! { #default_title.to_string() }
        };
        
        let detail_expr = if let Some(detail_val) = detail {
            quote! { #detail_val.to_string() }
        } else if let Some(error_fmt) = &error_format {
            // 如果有格式化的 error 属性，也可以用作 detail
            generate_format_expr(enum_ident, variant_ident, variant_fields, error_fmt)?
        } else {
            let default_detail = format!("{} occurred", variant_name);
            quote! { #default_detail.to_string() }
        };
        
        let mut builder = quote! {
            ::summer_web::problem_details::ProblemDetails::new(
                #custom_type,
                #title_expr,
                #status_code
            ).with_detail(#detail_expr)
        };
        
        if let Some(instance_val) = instance {
            builder = quote! { #builder.with_instance(#instance_val) };
        }
        
        builder
    } else {
        // 使用默认的状态码映射，problem_type 使用 about:blank
        match status_code {
            400 => {
                let detail_expr = if let Some(detail_val) = detail {
                    quote! { #detail_val.to_string() }
                } else if let Some(error_fmt) = &error_format {
                    generate_format_expr(enum_ident, variant_ident, variant_fields, error_fmt)?
                } else {
                    let default_detail = format!("{} occurred", variant_name);
                    quote! { #default_detail.to_string() }
                };
                quote! {
                    ::summer_web::problem_details::ProblemDetails::validation_error(#detail_expr)
                }
            },
            401 => quote! {
                ::summer_web::problem_details::ProblemDetails::authentication_error()
            },
            403 => quote! {
                ::summer_web::problem_details::ProblemDetails::authorization_error()
            },
            404 => {
                let resource_expr = if let Some(detail_val) = detail {
                    quote! { #detail_val.to_string() }
                } else if let Some(error_fmt) = &error_format {
                    generate_format_expr(enum_ident, variant_ident, variant_fields, error_fmt)?
                } else {
                    quote! { "resource".to_string() }
                };
                quote! {
                    ::summer_web::problem_details::ProblemDetails::not_found(#resource_expr)
                }
            },
            500 => quote! {
                ::summer_web::problem_details::ProblemDetails::internal_server_error()
            },
            503 => quote! {
                ::summer_web::problem_details::ProblemDetails::service_unavailable()
            },
            _ => {
                // 对于其他状态码，使用 about:blank 作为默认 problem_type
                let problem_type = "about:blank".to_string();
                
                let title_expr = if let Some(title_val) = title {
                    quote! { #title_val.to_string() }
                } else if let Some(error_fmt) = &error_format {
                    generate_format_expr(enum_ident, variant_ident, variant_fields, error_fmt)?
                } else {
                    let default_title = format!("{} Error", variant_name);
                    quote! { #default_title.to_string() }
                };
                
                let detail_expr = if let Some(detail_val) = detail {
                    quote! { #detail_val.to_string() }
                } else if let Some(error_fmt) = &error_format {
                    generate_format_expr(enum_ident, variant_ident, variant_fields, error_fmt)?
                } else {
                    let default_detail = format!("{} occurred", variant_name);
                    quote! { #default_detail.to_string() }
                };
                
                let mut builder = quote! {
                    ::summer_web::problem_details::ProblemDetails::new(
                        #problem_type,
                        #title_expr,
                        #status_code
                    ).with_detail(#detail_expr)
                };
                
                if let Some(instance_val) = instance {
                    builder = quote! { #builder.with_instance(#instance_val) };
                }
                
                builder
            }
        }
    };
    
    Ok(problem_details_expr)
}

fn get_problem_type_from_attrs(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    for attr in attrs {
        if attr.path().is_ident("problem_type") {
            let value: syn::LitStr = attr.parse_args()?;
            return Ok(Some(value.value()));
        }
    }
    Ok(None)
}

fn get_title_from_attrs(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    // 首先检查是否有 #[title("...")] 属性
    for attr in attrs {
        if attr.path().is_ident("title") {
            let value: syn::LitStr = attr.parse_args()?;
            return Ok(Some(value.value()));
        }
    }
    
    // 如果没有 title 属性，尝试从 #[error("...")] 属性中提取
    for attr in attrs {
        if attr.path().is_ident("error") {
            // 解析 error 属性的内容
            if let Ok(meta) = attr.parse_args::<syn::LitStr>() {
                let error_msg = meta.value();
                // 检查是否包含格式化参数，如果包含则不使用作为 title
                if error_msg.contains('{') && error_msg.contains('}') {
                    // 包含格式化参数，不适合直接作为 title
                    return Ok(None);
                }
                // 如果是简单的字符串字面量，使用它作为 title
                return Ok(Some(error_msg));
            }
        }
    }
    
    Ok(None)
}

fn get_detail_from_attrs(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    for attr in attrs {
        if attr.path().is_ident("detail") {
            let value: syn::LitStr = attr.parse_args()?;
            return Ok(Some(value.value()));
        }
    }
    Ok(None)
}

fn get_instance_from_attrs(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    for attr in attrs {
        if attr.path().is_ident("instance") {
            let value: syn::LitStr = attr.parse_args()?;
            return Ok(Some(value.value()));
        }
    }
    Ok(None)
}

fn get_error_format_from_attrs(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    for attr in attrs {
        if attr.path().is_ident("error") {
            // 检查是否是 transparent
            if let Ok(meta) = attr.parse_args::<syn::Ident>() {
                if meta == "transparent" {
                    return Ok(None);
                }
            }
            
            // 尝试解析为字符串字面量
            if let Ok(meta) = attr.parse_args::<syn::LitStr>() {
                let error_msg = meta.value();
                // 如果包含格式化参数，返回格式化字符串
                if error_msg.contains('{') && error_msg.contains('}') {
                    return Ok(Some(error_msg));
                }
            }
        }
    }
    Ok(None)
}

fn generate_format_expr(
    _enum_ident: &syn::Ident,
    _variant_ident: &syn::Ident,
    variant_fields: &Fields,
    format_str: &str,
) -> syn::Result<TokenStream> {
    match variant_fields {
        Fields::Unit => {
            // 单元变体，直接返回格式化字符串（不应该有参数）
            Ok(quote! { #format_str.to_string() })
        },
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            // 单个未命名字段，使用 inner 作为格式化参数
            Ok(quote! { format!(#format_str, inner) })
        },
        Fields::Unnamed(_) => {
            // 多个未命名字段，暂时不支持复杂格式化
            Ok(quote! { #format_str.to_string() })
        },
        Fields::Named(_) => {
            // 命名字段，暂时不支持复杂格式化
            Ok(quote! { #format_str.to_string() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_derive_basic() {
        let input: DeriveInput = syn::parse_quote! {
            #[derive(ProblemDetails)]
            pub enum TestErrors {
                #[status_code(400)]
                ValidationError,
                #[status_code(404)]
                NotFound,
            }
        };

        let result = expand_derive(input).unwrap();
        // Just check it compiles without panicking
        assert!(!result.is_empty());
        
        // Verify that the generated code includes both From and IntoResponse implementations
        let result_str = result.to_string();
        assert!(result_str.contains("impl From"));
        assert!(result_str.contains("impl :: summer_web :: axum :: response :: IntoResponse"));
    }

    #[test]
    fn test_expand_derive_with_custom_attributes() {
        let input: DeriveInput = syn::parse_quote! {
            #[derive(ProblemDetails)]
            pub enum TestErrors {
                #[status_code(400)]
                #[problem_type("https://example.com/problems/validation")]
                #[title("Custom Validation Error")]
                #[detail("Custom validation detail")]
                ValidationError,
            }
        };

        let result = expand_derive(input).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_get_problem_type_from_attrs() {
        let attrs: Vec<syn::Attribute> = vec![
            syn::parse_quote! { #[problem_type("https://example.com/problems/test")] }
        ];
        
        let result = get_problem_type_from_attrs(&attrs).unwrap();
        assert_eq!(result, Some("https://example.com/problems/test".to_string()));
    }

    #[test]
    fn test_get_title_from_attrs() {
        // 测试显式的 title 属性
        let attrs: Vec<syn::Attribute> = vec![
            syn::parse_quote! { #[title("Test Title")] }
        ];
        
        let result = get_title_from_attrs(&attrs).unwrap();
        assert_eq!(result, Some("Test Title".to_string()));
    }

    #[test]
    fn test_get_title_from_error_attr() {
        // 测试从 error 属性推导 title
        let attrs: Vec<syn::Attribute> = vec![
            syn::parse_quote! { #[error("Validation Failed")] }
        ];
        
        let result = get_title_from_attrs(&attrs).unwrap();
        assert_eq!(result, Some("Validation Failed".to_string()));
    }

    #[test]
    fn test_get_title_from_error_attr_with_params() {
        // 测试包含格式化参数的 error 属性不会被用作 title
        let attrs: Vec<syn::Attribute> = vec![
            syn::parse_quote! { #[error("Error occurred: {0:?}")] }
        ];
        
        let result = get_title_from_attrs(&attrs).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_error_format_from_attrs() {
        // 测试提取格式化的 error 属性
        let attrs: Vec<syn::Attribute> = vec![
            syn::parse_quote! { #[error("TeaPod error occurred: {0:?}")] }
        ];
        
        let result = get_error_format_from_attrs(&attrs).unwrap();
        assert_eq!(result, Some("TeaPod error occurred: {0:?}".to_string()));
    }

    #[test]
    fn test_generate_format_expr() {
        use syn::{Fields, FieldsUnnamed, Field};
        
        let enum_ident = syn::parse_quote! { TestEnum };
        let variant_ident = syn::parse_quote! { TestVariant };
        
        // 创建一个包含单个未命名字段的 Fields
        let field: Field = syn::parse_quote! { CustomErrorSchema };
        let mut unnamed = syn::punctuated::Punctuated::new();
        unnamed.push(field);
        let fields = Fields::Unnamed(FieldsUnnamed {
            paren_token: Default::default(),
            unnamed,
        });
        
        let result = generate_format_expr(&enum_ident, &variant_ident, &fields, "Error: {0:?}").unwrap();
        let expected = quote! { format!("Error: {0:?}", inner) };
        
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn test_title_precedence() {
        // 测试 title 属性优先于 error 属性
        let attrs: Vec<syn::Attribute> = vec![
            syn::parse_quote! { #[title("Explicit Title")] },
            syn::parse_quote! { #[error("Error Message")] }
        ];
        
        let result = get_title_from_attrs(&attrs).unwrap();
        assert_eq!(result, Some("Explicit Title".to_string()));
    }
}
