//! [![summer-rs](https://img.shields.io/github/stars/summer-rs/summer-rs)](https://summer-rs.github.io/docs/plugins/summer-xxl-job)
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]

//! Summer integration for [`xxljob-sdk-rs`].
//!
//! This plugin runs the application as an **executor** of an
//! xxl-job-admin / ratch-job compatible scheduling server. It builds the
//! executor client at startup, registers all handlers staged via
//! [`XxlJobConfigurator::add_xxl_handler`] on [`AppBuilder`], and exposes
//! the [`XxlClient`] as a Summer component for runtime usage.

pub mod config;

pub use config::XxlJobConfig;
pub use xxljob_sdk_rs;
pub use xxljob_sdk_rs::{
    AsyncJobHandler, JobContext, JobHandler, SyncJobHandler, XxlClient, XxlClientBuilder,
};

use std::ops::Deref;
use std::sync::Arc;

use summer::app::{App, AppBuilder};
use summer::async_trait;
use summer::config::ConfigRegistry;
use summer::error::Result as SummerResult;
use summer::plugin::component::ComponentRef;
use summer::plugin::ComponentRegistry;
use summer::plugin::MutableComponentRegistry;
use summer::{plugin::Plugin, signal};

/// Internal component used to collect handlers registered before the plugin
/// finishes its `build` phase. It is read by [`XxlJobPlugin`] during build.
#[derive(Clone, Default)]
pub struct XxlHandlerRegistry(Vec<(Arc<String>, JobHandler)>);

impl XxlHandlerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, name: Arc<String>, handler: JobHandler) {
        self.0.push((name, handler));
    }
}

impl Deref for XxlHandlerRegistry {
    type Target = Vec<(Arc<String>, JobHandler)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Extension trait on [`AppBuilder`] for staging xxl-job executor handlers.
///
/// The handler is held in an internal registry component; the plugin
/// performs the real `XxlClient::register` call when the application is
/// built.
pub trait XxlJobConfigurator {
    /// Stage an executor handler under the given `name`. The name is the
    /// `JobHandler` value configured on admin side.
    fn add_xxl_handler(&mut self, name: impl Into<String>, handler: JobHandler) -> &mut Self;

    /// Convenience wrapper for async handlers; wraps the value in `Arc` internally.
    fn add_xxl_async_handler<H>(&mut self, name: impl Into<String>, handler: H) -> &mut Self
    where
        H: AsyncJobHandler + 'static,
    {
        self.add_xxl_handler(name, JobHandler::Async(Arc::new(handler)))
    }

    /// Convenience wrapper for sync handlers; wraps the value in `Arc` internally.
    fn add_xxl_sync_handler<H>(&mut self, name: impl Into<String>, handler: H) -> &mut Self
    where
        H: SyncJobHandler + 'static,
    {
        self.add_xxl_handler(name, JobHandler::Sync(Arc::new(handler)))
    }
}

impl XxlJobConfigurator for AppBuilder {
    fn add_xxl_handler(&mut self, name: impl Into<String>, handler: JobHandler) -> &mut Self {
        let name = Arc::new(name.into());
        if let Some(reg) = self.get_component_ref::<XxlHandlerRegistry>() {
            // Same pattern as summer-job's `Jobs` aggregator: mutate the
            // singleton registry component in place.
            unsafe {
                let raw_ptr = ComponentRef::into_raw(reg);
                let reg = &mut *(raw_ptr as *mut XxlHandlerRegistry);
                reg.push(name, handler);
            }
            self
        } else {
            let mut reg = XxlHandlerRegistry::new();
            reg.push(name, handler);
            self.add_component(reg)
        }
    }
}

pub struct XxlJobPlugin;

#[async_trait]
impl Plugin for XxlJobPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let config = app
            .get_config::<XxlJobConfig>()
            .expect("xxl-job plugin config load failed");

        let client = Self::build_client(&config).expect("build xxl-job client failed");

        if let Some(reg) = app.get_component_ref::<XxlHandlerRegistry>() {
            for (name, handler) in reg.iter() {
                let name = name.clone();
                let handler = handler.clone();
                let display_name = name.clone();
                client
                    .register(name, handler)
                    .unwrap_or_else(|e| panic!("register xxl handler {display_name} failed: {e}"));
                tracing::info!(
                    handler = %display_name,
                    "registered xxl-job executor handler"
                );
            }
        } else {
            tracing::warn!("xxl-job plugin: no executor handler registered via add_xxl_handler");
        }

        // Expose the client as a Summer component (clone is cheap; it's an Arc internally).
        app.add_component(XxlClientHandle(client));

        // Block summer's run loop on ctrl_c / SIGTERM so the SDK's embedded
        // executor HTTP server keeps serving admin callbacks.
        app.add_scheduler(|_app: Arc<App>| Box::new(Self::schedule(_app)));

        app.add_shutdown_hook(move |_app| {
            Box::new(async move { Ok("xxl-job: client released".to_string()) })
        });
    }
}

impl XxlJobPlugin {
    async fn schedule(_app: Arc<App>) -> SummerResult<String> {
        signal::shutdown_signal("xxl-job").await;
        Ok("xxl-job executor stopped".to_string())
    }
    fn build_client(config: &XxlJobConfig) -> anyhow::Result<Arc<XxlClient>> {
        let mut builder = XxlClientBuilder::new(config.admin_addresses.clone())
            .set_app_name(config.app_name.clone())
            .set_log_path(config.log_path.clone())
            .set_ssl_danger_accept_invalid_certs(config.ssl_danger_accept_invalid_certs);

        if let Some(token) = config.access_token.clone() {
            builder = builder.set_access_token(token);
        }
        if let Some(ip) = config.ip.clone() {
            builder = builder.set_ip(ip);
        }
        if let Some(port) = config.port {
            builder = builder.set_port(port);
        }
        if let Some(days) = config.log_retention_days {
            builder = builder.set_log_retention_days(days);
        }
        if !config.headers.is_empty() {
            builder = builder.set_headers(config.headers.clone());
        }

        builder.build()
    }
}

/// Cloneable component wrapper around the SDK's `Arc<XxlClient>` so it can
/// be injected via Summer's component registry.
#[derive(Clone)]
pub struct XxlClientHandle(pub Arc<XxlClient>);

impl Deref for XxlClientHandle {
    type Target = XxlClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
