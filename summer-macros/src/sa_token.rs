//! Sa-Token authentication macros for summer-rs
//!
//! These macros provide annotation-style authentication similar to Java's Sa-Token.
//! They generate code that converts SaTokenError to summer-web's WebError.
//!
//! # Available Macros
//!
//! - `#[sa_check_login]` - Check if user is logged in
//! - `#[sa_check_role("role")]` - Check if user has specific role
//! - `#[sa_check_permission("permission")]` - Check if user has specific permission
//! - `#[sa_check_roles_and("role1", "role2")]` - Check if user has all specified roles
//! - `#[sa_check_roles_or("role1", "role2")]` - Check if user has any of specified roles
//! - `#[sa_check_permissions_and("perm1", "perm2")]` - Check if user has all specified permissions
//! - `#[sa_check_permissions_or("perm1", "perm2")]` - Check if user has any of specified permissions

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, punctuated::Punctuated, ItemFn, LitStr, Token};

/// Check login status macro
///
/// # Example
/// ```rust,ignore
/// #[sa_check_login]
/// async fn user_info() -> impl IntoResponse {
///     "User info"
/// }
/// ```
pub fn sa_check_login_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;
    let fn_body = &input.block;
    let fn_attrs = &input.attrs;
    let fn_vis = &input.vis;
    let fn_asyncness = &input.sig.asyncness;
    let fn_generics = &input.sig.generics;
    let fn_where_clause = &input.sig.generics.where_clause;

    if fn_asyncness.is_none() {
        return syn::Error::new_spanned(fn_name, "sa_check_login requires async function")
            .to_compile_error()
            .into();
    }

    let check_code = quote! {
        if !summer_sa_token::StpUtil::is_login_current() {
            return Err(summer_web::error::KnownWebError::unauthorized("Not logged in").into());
        }
    };

    let expanded: TokenStream2 = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_asyncness fn #fn_name #fn_generics(#fn_inputs) #fn_output #fn_where_clause {
            #check_code
            #fn_body
        }
    };

    expanded.into()
}

/// Check role macro
///
/// # Example
/// ```rust,ignore
/// #[sa_check_role("admin")]
/// async fn admin_panel() -> impl IntoResponse {
///     "Admin panel"
/// }
/// ```
pub fn sa_check_role_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let role = parse_macro_input!(attr as LitStr);
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;
    let fn_body = &input.block;
    let fn_attrs = &input.attrs;
    let fn_vis = &input.vis;
    let fn_asyncness = &input.sig.asyncness;
    let fn_generics = &input.sig.generics;
    let fn_where_clause = &input.sig.generics.where_clause;
    let role_value = role.value();

    if fn_asyncness.is_none() {
        return syn::Error::new_spanned(fn_name, "sa_check_role requires async function")
            .to_compile_error()
            .into();
    }

    let check_code = quote! {
        let __login_id = summer_sa_token::StpUtil::get_login_id_as_string()
            .await
            .map_err(|_| summer_web::error::KnownWebError::unauthorized("Not logged in"))?;
        if !summer_sa_token::StpUtil::has_role(&__login_id, #role_value).await {
            return Err(summer_web::error::KnownWebError::forbidden(
                format!("Missing required role: {}", #role_value)
            ).into());
        }
    };

    let expanded: TokenStream2 = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_asyncness fn #fn_name #fn_generics(#fn_inputs) #fn_output #fn_where_clause {
            #check_code
            #fn_body
        }
    };

    expanded.into()
}

/// Check permission macro
///
/// # Example
/// ```rust,ignore
/// #[sa_check_permission("user:delete")]
/// async fn delete_user() -> impl IntoResponse {
///     "User deleted"
/// }
/// ```
pub fn sa_check_permission_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let permission = parse_macro_input!(attr as LitStr);
    let perm_value = permission.value();

    if perm_value.trim().is_empty() {
        return syn::Error::new_spanned(&permission, "Permission identifier cannot be empty")
            .to_compile_error()
            .into();
    }

    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;
    let fn_body = &input.block;
    let fn_attrs = &input.attrs;
    let fn_vis = &input.vis;
    let fn_asyncness = &input.sig.asyncness;
    let fn_generics = &input.sig.generics;
    let fn_where_clause = &input.sig.generics.where_clause;

    if fn_asyncness.is_none() {
        return syn::Error::new_spanned(fn_name, "sa_check_permission requires async function")
            .to_compile_error()
            .into();
    }

    let check_code = quote! {
        let __login_id = summer_sa_token::StpUtil::get_login_id_as_string()
            .await
            .map_err(|_| summer_web::error::KnownWebError::unauthorized("Not logged in"))?;
        if !summer_sa_token::StpUtil::has_permission(&__login_id, #perm_value).await {
            return Err(summer_web::error::KnownWebError::forbidden(
                format!("Missing required permission: {}", #perm_value)
            ).into());
        }
    };

    let expanded: TokenStream2 = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_asyncness fn #fn_name #fn_generics(#fn_inputs) #fn_output #fn_where_clause {
            #check_code
            #fn_body
        }
    };

    expanded.into()
}

/// Check multiple roles with AND logic
///
/// # Example
/// ```rust,ignore
/// #[sa_check_roles_and("admin", "super")]
/// async fn super_admin() -> impl IntoResponse {
///     "Super admin"
/// }
/// ```
pub fn sa_check_roles_and_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let roles = parse_macro_input!(attr with Punctuated::<LitStr, Token![,]>::parse_terminated);
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;
    let fn_body = &input.block;
    let fn_attrs = &input.attrs;
    let fn_vis = &input.vis;
    let fn_asyncness = &input.sig.asyncness;
    let fn_generics = &input.sig.generics;
    let fn_where_clause = &input.sig.generics.where_clause;

    if fn_asyncness.is_none() {
        return syn::Error::new_spanned(fn_name, "sa_check_roles_and requires async function")
            .to_compile_error()
            .into();
    }

    let role_values: Vec<String> = roles.iter().map(|r| r.value()).collect();
    let role_checks = role_values.iter().map(|role| {
        quote! {
            if !summer_sa_token::StpUtil::has_role(&__login_id, #role).await {
                return Err(summer_web::error::KnownWebError::forbidden(
                    format!("Missing required role: {}", #role)
                ).into());
            }
        }
    });

    let check_code = quote! {
        let __login_id = summer_sa_token::StpUtil::get_login_id_as_string()
            .await
            .map_err(|_| summer_web::error::KnownWebError::unauthorized("Not logged in"))?;
        #(#role_checks)*
    };

    let expanded: TokenStream2 = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_asyncness fn #fn_name #fn_generics(#fn_inputs) #fn_output #fn_where_clause {
            #check_code
            #fn_body
        }
    };

    expanded.into()
}

/// Check multiple roles with OR logic
///
/// # Example
/// ```rust,ignore
/// #[sa_check_roles_or("admin", "manager")]
/// async fn management() -> impl IntoResponse {
///     "Management area"
/// }
/// ```
pub fn sa_check_roles_or_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let roles = parse_macro_input!(attr with Punctuated::<LitStr, Token![,]>::parse_terminated);
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;
    let fn_body = &input.block;
    let fn_attrs = &input.attrs;
    let fn_vis = &input.vis;
    let fn_asyncness = &input.sig.asyncness;
    let fn_generics = &input.sig.generics;
    let fn_where_clause = &input.sig.generics.where_clause;

    if fn_asyncness.is_none() {
        return syn::Error::new_spanned(fn_name, "sa_check_roles_or requires async function")
            .to_compile_error()
            .into();
    }

    let role_values: Vec<String> = roles.iter().map(|r| r.value()).collect();
    let role_checks = role_values.iter().map(|role| {
        quote! {
            if summer_sa_token::StpUtil::has_role(&__login_id, #role).await {
                __has_any_role = true;
            }
        }
    });

    let roles_str = role_values.join(", ");

    let check_code = quote! {
        let __login_id = summer_sa_token::StpUtil::get_login_id_as_string()
            .await
            .map_err(|_| summer_web::error::KnownWebError::unauthorized("Not logged in"))?;
        let mut __has_any_role = false;
        #(#role_checks)*
        if !__has_any_role {
            return Err(summer_web::error::KnownWebError::forbidden(
                format!("Missing any of required roles: {}", #roles_str)
            ).into());
        }
    };

    let expanded: TokenStream2 = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_asyncness fn #fn_name #fn_generics(#fn_inputs) #fn_output #fn_where_clause {
            #check_code
            #fn_body
        }
    };

    expanded.into()
}

/// Check multiple permissions with AND logic
///
/// # Example
/// ```rust,ignore
/// #[sa_check_permissions_and("user:read", "user:write")]
/// async fn user_rw() -> impl IntoResponse {
///     "User read/write"
/// }
/// ```
pub fn sa_check_permissions_and_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let permissions =
        parse_macro_input!(attr with Punctuated::<LitStr, Token![,]>::parse_terminated);
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;
    let fn_body = &input.block;
    let fn_attrs = &input.attrs;
    let fn_vis = &input.vis;
    let fn_asyncness = &input.sig.asyncness;
    let fn_generics = &input.sig.generics;
    let fn_where_clause = &input.sig.generics.where_clause;

    if fn_asyncness.is_none() {
        return syn::Error::new_spanned(
            fn_name,
            "sa_check_permissions_and requires async function",
        )
        .to_compile_error()
        .into();
    }

    let perm_values: Vec<String> = permissions.iter().map(|p| p.value()).collect();
    let perm_checks = perm_values.iter().map(|perm| {
        quote! {
            if !summer_sa_token::StpUtil::has_permission(&__login_id, #perm).await {
                return Err(summer_web::error::KnownWebError::forbidden(
                    format!("Missing required permission: {}", #perm)
                ).into());
            }
        }
    });

    let check_code = quote! {
        let __login_id = summer_sa_token::StpUtil::get_login_id_as_string()
            .await
            .map_err(|_| summer_web::error::KnownWebError::unauthorized("Not logged in"))?;
        #(#perm_checks)*
    };

    let expanded: TokenStream2 = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_asyncness fn #fn_name #fn_generics(#fn_inputs) #fn_output #fn_where_clause {
            #check_code
            #fn_body
        }
    };

    expanded.into()
}

/// Check multiple permissions with OR logic
///
/// # Example
/// ```rust,ignore
/// #[sa_check_permissions_or("admin:*", "user:delete")]
/// async fn delete() -> impl IntoResponse {
///     "Delete operation"
/// }
/// ```
pub fn sa_check_permissions_or_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let permissions =
        parse_macro_input!(attr with Punctuated::<LitStr, Token![,]>::parse_terminated);
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_inputs = &input.sig.inputs;
    let fn_output = &input.sig.output;
    let fn_body = &input.block;
    let fn_attrs = &input.attrs;
    let fn_vis = &input.vis;
    let fn_asyncness = &input.sig.asyncness;
    let fn_generics = &input.sig.generics;
    let fn_where_clause = &input.sig.generics.where_clause;

    if fn_asyncness.is_none() {
        return syn::Error::new_spanned(
            fn_name,
            "sa_check_permissions_or requires async function",
        )
        .to_compile_error()
        .into();
    }

    let perm_values: Vec<String> = permissions.iter().map(|p| p.value()).collect();
    let perm_checks = perm_values.iter().map(|perm| {
        quote! {
            if summer_sa_token::StpUtil::has_permission(&__login_id, #perm).await {
                __has_any_perm = true;
            }
        }
    });

    let perms_str = perm_values.join(", ");

    let check_code = quote! {
        let __login_id = summer_sa_token::StpUtil::get_login_id_as_string()
            .await
            .map_err(|_| summer_web::error::KnownWebError::unauthorized("Not logged in"))?;
        let mut __has_any_perm = false;
        #(#perm_checks)*
        if !__has_any_perm {
            return Err(summer_web::error::KnownWebError::forbidden(
                format!("Missing any of required permissions: {}", #perms_str)
            ).into());
        }
    };

    let expanded: TokenStream2 = quote! {
        #(#fn_attrs)*
        #fn_vis #fn_asyncness fn #fn_name #fn_generics(#fn_inputs) #fn_output #fn_where_clause {
            #check_code
            #fn_body
        }
    };

    expanded.into()
}

/// Ignore authentication for this endpoint
///
/// This macro marks an endpoint to skip authentication checks,
/// even if it's under a path that normally requires authentication.
///
/// # Example
/// ```rust,ignore
/// #[sa_ignore]
/// async fn public_endpoint() -> impl IntoResponse {
///     "This endpoint is public"
/// }
/// ```
pub fn sa_ignore_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // sa_ignore is a marker - just pass through the function unchanged
    // The actual ignore logic is handled by the middleware checking for this attribute
    item
}