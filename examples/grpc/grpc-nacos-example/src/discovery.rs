use anyhow::{bail, Context};
use summer::config::Configurable;
use summer_nacos::NacosNamingService;
use tonic::transport::channel::{Change, Endpoint};
use tonic::transport::Channel;

#[derive(Clone, serde::Deserialize, Configurable)]
#[config_prefix = "discovery"]
pub struct DiscoveryConfig {
    pub service_name: String,
    #[serde(default = "default_group")]
    pub group: String,
}

fn default_group() -> String {
    "DEFAULT_GROUP".to_string()
}

/// Lists tonic URIs for healthy Nacos instances (prefers `metadata.protocol = grpc`).
pub async fn grpc_endpoints(
    naming: &NacosNamingService,
    config: &DiscoveryConfig,
) -> anyhow::Result<Vec<String>> {
    let instances = naming
        .get_all_instances(
            config.service_name.clone(),
            Some(config.group.clone()),
            vec![],
            true,
        )
        .await
        .with_context(|| {
            format!(
                "nacos get instances for {}@{}",
                config.service_name, config.group
            )
        })?;

    let mut picked: Vec<_> = instances
        .iter()
        .filter(|inst| {
            inst.metadata
                .get("protocol")
                .is_some_and(|p| p == "grpc")
        })
        .collect();
    if picked.is_empty() {
        picked = instances.iter().collect();
    }
    if picked.is_empty() {
        bail!(
            "no instance for service {} in group {}",
            config.service_name,
            config.group
        );
    }

    Ok(picked
        .into_iter()
        .map(|inst| format!("http://{}:{}", inst.ip, inst.port))
        .collect())
}

/// Builds one [`Channel`] that load-balances across all discovered endpoints (tonic P2C).
pub async fn connect_balanced(endpoints: Vec<String>) -> anyhow::Result<Channel> {
    let (channel, tx) = Channel::balance_channel(endpoints.len().max(1));
    for (key, uri) in endpoints.into_iter().enumerate() {
        let endpoint = Endpoint::from_shared(uri.clone())
            .with_context(|| format!("invalid endpoint {uri}"))?;
        tx.send(Change::Insert(key, endpoint))
            .await
            .map_err(|_| anyhow::anyhow!("tonic balance channel closed"))?;
    }
    Ok(channel)
}
