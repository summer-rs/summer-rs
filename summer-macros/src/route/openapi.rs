use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Attribute, Expr, ExprLit, Lit, Meta, MetaNameValue};

/// Describes a single API operation on a path.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct OperationMetadata {
    /// A list of tags for API documentation control.
    /// Tags can be used for logical grouping of operations
    /// by resources or any other qualifier.
    pub tags: Vec<String>,
    /// A short summary of what the operation does.
    pub summary: Option<String>,
    /// A verbose explanation of the operation behavior.
    /// CommonMark syntax MAY be used for rich text representation.
    pub description: Option<String>,
    /// Additional external documentation for this operation.
    pub external_docs: Option<ExternalDocumentation>,
    /// Unique string used to identify the operation.
    /// The id MUST be unique among all operations described in the API.
    /// Tools and libraries MAY use the operationId to uniquely identify
    /// an operation, therefore, it is RECOMMENDED to follow common
    /// programming naming conventions.
    pub operation_id: Option<String>,
    /// Declares this operation to be deprecated.Default value is false.
    pub deprecated: bool,
    /// Additional status codes that can be returned by this operation.
    pub status_codes: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ExternalDocumentation {
    /// A description of the target documentation.
    /// CommonMark syntax MAY be used for rich text representation.
    pub description: Option<String>,
    /// REQUIRED. The URL for the target documentation.
    /// This MUST be in the format of a URL.
    pub url: String,
}

impl ToTokens for OperationMetadata {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let tags = {
            let items = self.tags.iter().map(|tag| quote! { #tag.to_string() });
            quote! { vec![#(#items),*] }
        };

        let summary = match &self.summary {
            Some(s) => quote! { Some(#s.to_string()) },
            None => quote! { None },
        };

        let description = match &self.description {
            Some(d) => quote! { Some(#d.to_string()) },
            None => quote! { None },
        };

        let operation_id = match &self.operation_id {
            Some(id) => quote! { Some(#id.to_string()) },
            None => quote! { None },
        };

        let deprecated = self.deprecated;

        let external_docs = if let Some(doc) = &self.external_docs {
            let desc = match &doc.description {
                Some(s) => quote! { Some(#s.to_string()) },
                None => quote! { None },
            };
            let url = &doc.url;
            quote! {
                Some(::summer_web::aide::openapi::ExternalDocumentation {
                    description: #desc,
                    url: #url.to_string(),
                    ..Default::default()
                })
            }
        } else {
            quote! { None }
        };

        tokens.extend(quote! {
            ::summer_web::aide::openapi::Operation {
                tags: #tags,
                summary: #summary,
                description: #description,
                operation_id: #operation_id,
                deprecated: #deprecated,
                external_docs: #external_docs,
                ..Default::default()
            }
        });
    }
}

pub fn parse_doc_attributes(attrs: &[syn::Attribute], fn_name: &str) -> OperationMetadata {
    let mut summary = None;
    let mut description = String::new();
    let mut tags = Vec::new();
    let mut id = Some(fn_name.to_string()); // default value = fn_name
    let mut deprecated = false;
    let mut external_docs = None;
    let mut status_codes = Vec::new();

    for (index, raw_line) in extract_doc_lines(attrs).into_iter().enumerate() {
        let line = raw_line.trim();
        if index == 0 && summary.is_none() && !line.is_empty() {
            summary = Some(line.trim_start_matches('#').trim_start().to_string());
        } else if let Some(stripped) = line.strip_prefix("@tag ") {
            tags.push(stripped.trim().to_string());
        } else if let Some(stripped) = line.strip_prefix("@id ") {
            id = Some(stripped.trim().to_string());
        } else if let Some(stripped) = line.strip_prefix("@see ") {
            external_docs = Some(ExternalDocumentation {
                url: stripped.trim().to_string(),
                description: None,
            });
        } else if line.starts_with("@deprecated") {
            deprecated = true;
        } else if let Some(stripped) = line.strip_prefix("@status_codes ") {
            status_codes.extend(
                stripped
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
            );
        } else if !line.starts_with('@') {
            description.push_str(line);
            description.push('\n');
        }
    }

    let description = description.trim();

    OperationMetadata {
        summary,
        description: if description.is_empty() {
            None
        } else {
            Some(description.to_string())
        },
        tags,
        operation_id: id,
        deprecated,
        external_docs,
        status_codes,
    }
}

pub fn extract_doc_lines(attrs: &[Attribute]) -> Vec<String> {
    let mut lines = Vec::new();

    for attr in attrs {
        // 只处理 #[doc = "..."] 类型
        if let Meta::NameValue(MetaNameValue { path, value, .. }) = &attr.meta {
            if path.is_ident("doc") {
                // value 是一个 Expr
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = value
                {
                    let line = s.value().trim_start().to_string();
                    lines.push(line);
                }
            }
        }
    }

    while matches!(lines.first(), Some(l) if l.trim().is_empty()) {
        lines.remove(0);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_doc_lines() {
        let input: syn::ItemFn = syn::parse_quote! {
            ///
            /// # 创建一个新的待办事项
            /// 返回创建结果
            fn create_todo() {}
        };

        let docs = extract_doc_lines(&input.attrs);
        assert_eq!(docs, vec!["# 创建一个新的待办事项", "返回创建结果"]);
    }

    #[test]
    fn test_parse_doc_attributes_single_line() {
        let item: syn::ItemFn = syn::parse_quote! {
            /// # 获取任务列表
            fn list_todos() {}
        };

        let meta = parse_doc_attributes(&item.attrs, "list_todos");
        assert_eq!(
            meta,
            OperationMetadata {
                operation_id: Some("list_todos".into()),
                summary: Some("获取任务列表".into()),
                status_codes: Vec::new(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_parse_doc_attributes() {
        let item: syn::ItemFn = syn::parse_quote! {
            /// # 创建一个新的待办事项
            /// 此接口用于新增待办项
            fn create_todo() {}
        };

        // 调用解析函数
        let meta = parse_doc_attributes(&item.attrs, "create_todo");

        // 断言结果
        assert_eq!(
            meta,
            OperationMetadata {
                operation_id: Some("create_todo".into()),
                summary: Some("创建一个新的待办事项".into()),
                description: Some("此接口用于新增待办项".into()),
                status_codes: Vec::new(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_parse_doc_with_tags_and_deprecated() {
        let item: syn::ItemFn = syn::parse_quote! {
            /// # 创建一个新的待办事项
            /// 此接口用于新增待办项
            /// @tag todo
            /// @tag create
            /// @deprecated
            fn create_todo() {}
        };

        let meta = parse_doc_attributes(&item.attrs, "create_todo");

        assert_eq!(
            meta,
            OperationMetadata {
                operation_id: Some("create_todo".into()),
                tags: vec!["todo".into(), "create".into()],
                summary: Some("创建一个新的待办事项".into()),
                description: Some("此接口用于新增待办项".into()),
                deprecated: true,
                status_codes: Vec::new(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_parse_doc_with_single_tags() {
        let item: syn::ItemFn = syn::parse_quote! {
            /// # 获取任务列表
            /// @tag list
            fn list_todos() {}
        };

        let meta = parse_doc_attributes(&item.attrs, "list_todos");
        assert_eq!(
            meta,
            OperationMetadata {
                operation_id: Some("list_todos".into()),
                tags: vec!["list".into()],
                summary: Some("获取任务列表".into()),
                status_codes: Vec::new(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_parse_doc_with_status_codes() {
        let item: syn::ItemFn = syn::parse_quote! {
            /// Protected user info api
            /// api description detail
            /// @tag user
            /// @status_codes Errors::A
            fn protected_user_info_api() {}
        };

        let meta = parse_doc_attributes(&item.attrs, "protected_user_info_api");
        assert_eq!(
            meta,
            OperationMetadata {
                operation_id: Some("protected_user_info_api".into()),
                tags: vec!["user".into()],
                summary: Some("Protected user info api".into()),
                description: Some("api description detail".into()),
                status_codes: vec!["Errors::A".into()],
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_parse_doc_with_multiple_status_codes() {
        let item: syn::ItemFn = syn::parse_quote! {
            /// Test API
            /// @status_codes Errors::A, Errors::B
            /// @status_codes Errors::C
            fn test_api() {}
        };

        let meta = parse_doc_attributes(&item.attrs, "test_api");
        assert_eq!(
            meta,
            OperationMetadata {
                operation_id: Some("test_api".into()),
                summary: Some("Test API".into()),
                status_codes: vec!["Errors::A".into(), "Errors::B".into(), "Errors::C".into()],
                ..Default::default()
            }
        );
    }
}
