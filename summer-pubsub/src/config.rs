use anyhow::Context;
use google_cloud_auth::credentials::{
    Credentials, external_account, impersonated, service_account, user_account,
};
use google_cloud_auth::credentials::AccessTokenCredentials;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use summer::config::Configurable;

summer::submit_config_schema!("pubsub", PubSubConfig);

#[derive(Debug, Configurable, Clone, JsonSchema, Deserialize)]
#[config_prefix = "pubsub"]
pub struct PubSubConfig {
    /// When false, the plugin does not connect to Pub/Sub or register consumers.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// GCP project id used to expand short subscription and topic names.
    pub project_id: String,
    /// Optional API endpoint (for example the Pub/Sub emulator).
    pub endpoint: Option<String>,
    /// Optional path to a service account or ADC JSON key file.
    pub credentials: Option<String>,
}

fn default_enabled() -> bool {
    true
}

pub fn credentials_from_json_value(json: Value) -> anyhow::Result<Credentials> {
    let cred_type = json
        .get("type")
        .and_then(|v| v.as_str())
        .context("credential JSON missing string field `type`")?;
    let atc: AccessTokenCredentials = match cred_type {
        "authorized_user" => user_account::Builder::new(json)
            .build_access_token_credentials()
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        "service_account" => service_account::Builder::new(json)
            .build_access_token_credentials()
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        "external_account" => external_account::Builder::new(json)
            .build_access_token_credentials()
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        "impersonated_service_account" => impersonated::Builder::new(json)
            .build_access_token_credentials()
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        other => anyhow::bail!("unsupported credential type: {other}"),
    };
    Ok(atc.into())
}

pub fn credentials_from_file(path: &str) -> anyhow::Result<Credentials> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("read pubsub credentials file {path}"))?;
    let json: Value = serde_json::from_str(&raw).context("parse credentials JSON")?;
    credentials_from_json_value(json)
}
