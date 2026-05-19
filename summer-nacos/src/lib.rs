//! [![summer-rs](https://img.shields.io/github/stars/summer-rs/summer-rs)](https://summer-rs.github.io/docs/plugins/summer-nacos)
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]

pub mod config;

pub use config::{NacosBootstrapConfig, NacosConfig, NacosRegistrationConfig};
pub use nacos_sdk;

use anyhow::Context;
use nacos_sdk::api::config::{ConfigService, ConfigServiceBuilder};
use nacos_sdk::api::naming::{NamingService, NamingServiceBuilder, ServiceInstance};
use nacos_sdk::api::props::ClientProps;
use std::any::Any;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use summer::async_trait;
use summer::config::ConfigRegistry;
use summer::event::{
    AppEventListener, AppEventSubscriber, BuilderEventListener, ConfigEvent, EventSubscriber,
    WebServerStartedEvent,
};
use summer::plugin::ComponentRegistry;
use summer::plugin::MutableComponentRegistry;
use summer::{app::AppBuilder, error::Result, plugin::Plugin};

/// Nacos config center client (long-lived; clone is cheap).
pub type NacosConfigService = ConfigService;

/// Nacos naming client (long-lived; clone is cheap).
pub type NacosNamingService = NamingService;

pub struct NacosPlugin;

struct NacosConfigEventListener;

#[async_trait]
impl BuilderEventListener for NacosConfigEventListener {
    async fn on_event(
        &self,
        _event: Arc<dyn Any + Send + Sync>,
        app: &mut AppBuilder,
    ) -> Result<()> {
        NacosPlugin::on_config_event(app).await
    }
}

struct NacosWebRegistrationListener {
    naming: NamingService,
    reg: config::NacosRegistrationConfig,
    registered: Arc<Mutex<Option<ServiceInstance>>>,
}

#[async_trait]
impl AppEventListener for NacosWebRegistrationListener {
    async fn on_event(
        &self,
        event: Arc<dyn Any + Send + Sync>,
        _app: &summer::app::App,
    ) -> Result<()> {
        let event = event
            .downcast::<WebServerStartedEvent>()
            .expect("event listener received unexpected event type");
        let instance = build_service_instance(&self.reg, event.addr)?;
        self.naming
            .batch_register_instance(
                self.reg.service_name.clone(),
                Some(self.reg.group.clone()),
                vec![instance.clone()],
            )
            .await
            .context("nacos register instance")?;
        *self
            .registered
            .lock()
            .expect("registration lock poisoned") = Some(instance);
        tracing::info!(
            service = %self.reg.service_name,
            group = %self.reg.group,
            addr = %event.addr,
            "registered instance to nacos"
        );
        Ok(())
    }
}

#[async_trait]
impl Plugin for NacosPlugin {
    fn immediately(&self) -> bool {
        true
    }

    fn immediately_build(&self, app: &mut AppBuilder) {
        app.listen_dyn::<ConfigEvent>(Arc::new(NacosConfigEventListener));
    }
}

impl NacosPlugin {
    async fn on_config_event(app: &mut AppBuilder) -> Result<()> {
        let nacos = app.get_config::<NacosConfig>()?;

        if nacos.enable_config {
            let config_service = Self::build_config_service(&nacos).await?;
            for item in &nacos.bootstrap {
                let resp = config_service
                    .get_config(item.data_id.clone(), item.group.clone())
                    .await
                    .with_context(|| {
                        format!(
                            "fetch nacos config data_id={} group={}",
                            item.data_id, item.group
                        )
                    })?;
                app.merge_config_str(resp.content())?;
                tracing::info!(
                    data_id = %item.data_id,
                    group = %item.group,
                    "merged nacos config into application registry"
                );
            }
            app.add_component(config_service);
        }

        let nacos = app.get_config::<NacosConfig>()?;

        if nacos.enable_naming || nacos.registration.is_some() {
            let naming_service = Self::build_naming_service(&nacos).await?;
            if let Some(reg) = nacos.registration.clone() {
                Self::wire_registration(app, naming_service.clone(), reg);
            }
            app.add_component(naming_service);
        }

        Ok(())
    }

    fn client_props(config: &NacosConfig) -> ClientProps {
        let mut props = ClientProps::new()
            .server_addr(config.server_addr.clone())
            .namespace(config.namespace.clone())
            .app_name(config.app_name.clone());
        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            props = props.auth_username(username).auth_password(password);
        }
        props
    }

    async fn build_config_service(config: &NacosConfig) -> Result<ConfigService> {
        let mut builder = ConfigServiceBuilder::new(Self::client_props(config));
        if config.enable_auth {
            builder = builder.enable_auth_plugin_http();
        }
        builder
            .build()
            .await
            .context("build nacos config service")
            .map_err(Into::into)
    }

    async fn build_naming_service(config: &NacosConfig) -> Result<NamingService> {
        let mut builder = NamingServiceBuilder::new(Self::client_props(config));
        if config.enable_auth {
            builder = builder.enable_auth_plugin_http();
        }
        builder
            .build()
            .await
            .context("build nacos naming service")
            .map_err(Into::into)
    }

    fn wire_registration(
        app: &mut AppBuilder,
        naming: NamingService,
        reg: config::NacosRegistrationConfig,
    ) {
        let registered = Arc::new(Mutex::new(None::<ServiceInstance>));
        let registered_for_hook = registered.clone();
        let reg_for_hook = reg.clone();

        app.listen_app_dyn::<WebServerStartedEvent>(Arc::new(NacosWebRegistrationListener {
            naming,
            reg,
            registered,
        }));

        app.add_shutdown_hook(move |app| {
            let registered = registered_for_hook.clone();
            let reg = reg_for_hook.clone();
            Box::new(async move {
                let Some(instance) = registered
                    .lock()
                    .expect("registration lock poisoned")
                    .take()
                else {
                    return Ok("nacos: no instance to deregister".to_string());
                };
                let naming = app.get_expect_component::<NamingService>();
                naming
                    .deregister_instance(
                        reg.service_name.clone(),
                        Some(reg.group.clone()),
                        instance,
                    )
                    .await
                    .context("nacos deregister instance")?;
                Ok(format!(
                    "nacos: deregistered {}@{}",
                    reg.service_name, reg.group
                ))
            })
        });
    }
}

fn build_service_instance(
    reg: &config::NacosRegistrationConfig,
    bound: std::net::SocketAddr,
) -> Result<ServiceInstance> {
    let ip = reg
        .ip
        .clone()
        .or_else(|| Some(bound.ip().to_string()))
        .or_else(resolve_local_ip)
        .context("nacos registration requires ip (set nacos.registration.ip or run with summer-web)")?;
    let port = reg.port.unwrap_or(bound.port());

    let mut instance = ServiceInstance {
        ip,
        port: i32::from(port),
        weight: reg.weight,
        ..Default::default()
    };
    if let Some(cluster) = &reg.cluster {
        instance.cluster_name = Some(cluster.clone());
    }
    if !reg.metadata.is_empty() {
        instance.metadata = reg.metadata.clone();
    }
    Ok(instance)
}

#[cfg(feature = "naming")]
fn resolve_local_ip() -> Option<String> {
    local_ip_address::local_ip()
        .ok()
        .map(|ip| match ip {
            IpAddr::V4(v4) => v4.to_string(),
            IpAddr::V6(v6) => v6.to_string(),
        })
}

#[cfg(not(feature = "naming"))]
fn resolve_local_ip() -> Option<String> {
    None
}
