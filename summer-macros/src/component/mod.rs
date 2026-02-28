//! Component macro implementation
//!
//! This module provides the `#[component]` procedural macro for declarative
//! component registration in summer-rs applications.

mod attrs;
mod codegen;
mod dependency;
mod utils;
mod validation;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Main entry point for the `#[component]` macro
///
/// # Example
///
/// ```ignore
/// #[component]
/// fn create_db_connection(
///     Config(config): Config<DbConfig>,
/// ) -> DbConnection {
///     DbConnection::new(&config)
/// }
/// ```
pub fn component_macro(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse attribute arguments
    let attrs = match attrs::parse_component_attrs(attr.into()) {
        Ok(attrs) => attrs,
        Err(e) => return e.to_compile_error().into(),
    };

    // Parse function definition
    let func = parse_macro_input!(item as ItemFn);

    // Validate function signature
    if let Err(e) = validation::validate_function_signature(&func) {
        return e.to_compile_error().into();
    }

    // Analyze dependencies
    let dependencies = dependency::analyze_dependencies(&func);

    // Generate Plugin implementation
    let plugin_impl = codegen::generate_plugin_impl(&attrs, &func, &dependencies);

    // Generate inventory registration
    let inventory_submit = codegen::generate_inventory_submit(&attrs, &func);

    // Preserve original function
    let output = quote! {
        #plugin_impl
        #inventory_submit
        #func
    };

    output.into()
}
