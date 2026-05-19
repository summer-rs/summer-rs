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
    ServerProtocol, ServerStartedEvent,
};
use summer::plugin::ComponentRegistry;
use summer::plugin::MutableComponentRegistry;
use summer::{app::AppBuilder, error::Result, plugin::Plugin};

/// Nacos config center client (long-lived; clone is cheap).
pub type NacosConfigService = ConfigService;

/// Nacos naming client (long-lived; clone is cheap).
pub type NacosNamingService = NamingService;

pub struct NacosPlugin;

/// Pulls remote config on [`ConfigEvent`] during app build (before LogPlugin).
struct NacosConfigEventListener;

#[async_trait]
impl BuilderEventListener for NacosConfigEventListener {
    async fn on_event(
        &self,
        _event: Arc<dyn Any + Send + Sync>,
        app: &mut AppBuilder,
    ) -> Result<()> {
        NacosPlugin::on_config_event(app).await;
        Ok(())
    }
}

/// Registers the app to Nacos on each [`ServerStartedEvent`] (e.g. HTTP and gRPC).
struct NacosRegistrationListener {
    naming: NamingService,
    reg: config::NacosRegistrationConfig,
    /// Instances registered in this process; all are deregistered on shutdown.
    registered: Arc<Mutex<Vec<ServiceInstance>>>,
}

#[async_trait]
impl AppEventListener for NacosRegistrationListener {
    async fn on_event(
        &self,
        event: Arc<dyn Any + Send + Sync>,
        _app: &summer::app::App,
    ) -> Result<()> {
        let event = event
            .downcast::<ServerStartedEvent>()
            .expect("event listener received unexpected event type");
        let instance = build_service_instance(&self.reg, event.addr, event.protocol)?;
        self.naming
            .batch_register_instance(
                self.reg.service_name.clone(),
                Some(self.reg.group.clone()),
                vec![instance.clone()],
            )
            .await
            .context("nacos register instance")?;
        tracing::info!(
            service = %self.reg.service_name,
            group = %self.reg.group,
            protocol = %event.protocol.as_str(),
            ip = %instance.ip,
            port = instance.port,
            "registered instance to nacos"
        );
        // One entry per protocol (summer-web / summer-grpc each publish ServerStartedEvent).
        self.registered
            .lock()
            .expect("registration lock poisoned")
            .push(instance);
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
    /// Best-effort: logs failures but does not block application startup.
    async fn on_config_event(app: &mut AppBuilder) {
        let nacos = match app.get_config::<NacosConfig>() {
            Ok(nacos) => nacos,
            Err(e) => {
                tracing::error!(error = %e, "nacos: failed to load plugin config");
                return;
            }
        };

        if nacos.enable_config {
            match Self::build_config_service(&nacos).await {
                Err(e) => tracing::error!(error = %e, "nacos: failed to build config service"),
                Ok(config_service) => {
                    for item in &nacos.bootstrap {
                        let data_id = item.data_id.clone();
                        let group = item.group.clone();
                        let resp = match config_service
                            .get_config(data_id.clone(), group.clone())
                            .await
                        {
                            Ok(resp) => resp,
                            Err(e) => {
                                tracing::error!(
                                    data_id = %data_id,
                                    group = %group,
                                    error = %e,
                                    "nacos: bootstrap config fetch failed"
                                );
                                continue;
                            }
                        };
                        if let Err(e) = app.merge_config_str(resp.content()) {
                            tracing::error!(
                                data_id = %data_id,
                                group = %group,
                                error = %e,
                                "nacos: bootstrap config merge failed"
                            );
                            continue;
                        }
                        tracing::info!(
                            data_id = %data_id,
                            group = %group,
                            "merged nacos config into application registry"
                        );
                    }
                    app.add_component(config_service);
                }
            }
        }

        let nacos = match app.get_config::<NacosConfig>() {
            Ok(nacos) => nacos,
            Err(e) => {
                tracing::error!(error = %e, "nacos: failed to reload config after bootstrap merge");
                return;
            }
        };

        if nacos.enable_naming || nacos.registration.is_some() {
            match Self::build_naming_service(&nacos).await {
                Err(e) => tracing::error!(error = %e, "nacos: failed to build naming service"),
                Ok(naming_service) => {
                    if let Some(reg) = nacos.registration.clone() {
                        Self::wire_registration(app, naming_service.clone(), reg);
                    }
                    app.add_component(naming_service);
                }
            }
        }
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

    /// Subscribes to [`ServerStartedEvent`] and deregisters every instance recorded above on shutdown.
    fn wire_registration(
        app: &mut AppBuilder,
        naming: NamingService,
        reg: config::NacosRegistrationConfig,
    ) {
        let registered = Arc::new(Mutex::new(Vec::<ServiceInstance>::new()));
        let registered_for_hook = registered.clone();
        let reg_for_hook = reg.clone();

        app.listen_app_dyn::<ServerStartedEvent>(Arc::new(NacosRegistrationListener {
            naming,
            reg,
            registered,
        }));

        app.add_shutdown_hook(move |app| {
            let registered = registered_for_hook.clone();
            let reg = reg_for_hook.clone();
            Box::new(async move {
                // Drain so repeated shutdown does not double-deregister.
                let instances = registered
                    .lock()
                    .expect("registration lock poisoned")
                    .drain(..)
                    .collect::<Vec<_>>();
                if instances.is_empty() {
                    return Ok("nacos: no instance to deregister".to_string());
                }
                let naming = app.get_expect_component::<NamingService>();
                let count = instances.len();
                for instance in instances {
                    naming
                        .deregister_instance(
                            reg.service_name.clone(),
                            Some(reg.group.clone()),
                            instance,
                        )
                        .await
                        .context("nacos deregister instance")?;
                }
                Ok(format!(
                    "nacos: deregistered {count} instance(s) of {}",
                    reg.service_name
                ))
            })
        });
    }
}

/// Builds a Nacos instance from config and the address reported by the server plugin.
fn build_service_instance(
    reg: &config::NacosRegistrationConfig,
    bound: std::net::SocketAddr,
    protocol: ServerProtocol,
) -> Result<ServiceInstance> {
    let ip = reg
        .ip
        .clone()
        .or_else(|| ip_from_bound(bound))
        .or_else(resolve_local_ip)
        .context("nacos registration requires ip (set nacos.registration.ip or run with summer-web/summer-grpc)")?;
    // Omit `registration.port` when using multiple protocols so each event keeps its own port.
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
    instance.metadata = reg.metadata.clone();
    // Config metadata wins; otherwise tag the instance with http / grpc for discovery filters.
    instance
        .metadata
        .entry("protocol".to_string())
        .or_insert_with(|| protocol.as_str().to_string());
    Ok(instance)
}

/// Returns the bound address IP unless it is unspecified (`0.0.0.0` / `::`).
fn ip_from_bound(bound: std::net::SocketAddr) -> Option<String> {
    let ip = bound.ip();
    if ip.is_unspecified() {
        None
    } else {
        Some(ip.to_string())
    }
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

#[cfg(test)]
mod tests {
    use super::{build_service_instance, ip_from_bound};
    use crate::config::NacosRegistrationConfig;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
    use summer::event::ServerProtocol;

    #[test]
    fn ip_from_bound_skips_unspecified_v4() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080);
        assert_eq!(ip_from_bound(addr), None);
    }

    #[test]
    fn ip_from_bound_skips_unspecified_v6() {
        let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 8080);
        assert_eq!(ip_from_bound(addr), None);
    }

    #[test]
    fn ip_from_bound_uses_concrete_ip() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)), 3000);
        assert_eq!(ip_from_bound(addr).as_deref(), Some("10.0.0.5"));
    }

    #[test]
    fn build_service_instance_sets_protocol_metadata() {
        let reg = NacosRegistrationConfig {
            service_name: "svc".to_string(),
            ..Default::default()
        };
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9090);
        let instance =
            build_service_instance(&reg, addr, ServerProtocol::Grpc).expect("build instance");
        assert_eq!(instance.metadata.get("protocol").map(String::as_str), Some("grpc"));
    }

    #[test]
    fn build_service_instance_config_metadata_overrides_protocol() {
        let mut reg = NacosRegistrationConfig {
            service_name: "svc".to_string(),
            ..Default::default()
        };
        reg.metadata.insert("protocol".into(), "custom".into());
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let instance =
            build_service_instance(&reg, addr, ServerProtocol::Http).expect("build instance");
        assert_eq!(instance.metadata.get("protocol").map(String::as_str), Some("custom"));
    }
}
