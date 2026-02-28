#![deny(missing_docs)]
//! For the complete documentation of summer, please click this address: [https://summer-rs.github.io]
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]
#![doc = include_str!("../../README.md")]

/// App Builder
pub mod app;
/// Banner
pub mod banner;
/// Config System:
pub mod config;
/// summer-rs definition error
pub mod error;
/// summer-rs extractor
pub mod extractor;
/// The log plugin is a built-in plugin of summer-rs and is also the first plugin loaded when the application starts.
pub mod log;
/// Plugin system: Through the documentation of this module you will learn how to implement your own plugins
pub mod plugin;
/// signal, such as "ctrl-c" notification
pub mod signal;

pub use app::App;
pub use async_trait::async_trait;
pub use summer_macros::auto_config;
pub use summer_macros::component;
pub use tracing;
pub use tracing_error::SpanTrace;
pub use inventory::submit as submit_inventory;