use crate::consumer::resolve_topic;
use anyhow::Context;
use bytes::Bytes;
use google_cloud_pubsub::client::Publisher;
use google_cloud_pubsub::model::Message as GcpMessage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Publishes to Pub/Sub topics, caching one [`Publisher`] per fully-qualified topic name.
#[derive(Clone)]
pub struct PubSubProducer {
    inner: Arc<Inner>,
}

struct Inner {
    project_id: String,
    endpoint: Option<String>,
    publishers: Mutex<HashMap<String, Publisher>>,
}

impl PubSubProducer {
    pub(crate) fn new(project_id: String, endpoint: Option<String>) -> Self {
        Self {
            inner: Arc::new(Inner {
                project_id,
                endpoint,
                publishers: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Publish a fully constructed [`GcpMessage`] to `topic` (short id or full resource name).
    pub async fn publish(&self, topic: &str, message: GcpMessage) -> anyhow::Result<String> {
        let publisher = self.publisher_for(topic).await?;
        publisher
            .publish(message)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Convenience helper: UTF-8 payload.
    pub async fn publish_utf8(
        &self,
        topic: &str,
        data: impl AsRef<str>,
    ) -> anyhow::Result<String> {
        self.publish(
            topic,
            GcpMessage::new().set_data(Bytes::copy_from_slice(data.as_ref().as_bytes())),
        )
        .await
    }

    async fn publisher_for(&self, topic: &str) -> anyhow::Result<Publisher> {
        let full = resolve_topic(self.inner.project_id.as_str(), topic);
        let mut map = self.inner.publishers.lock().await;
        if let Some(p) = map.get(&full) {
            return Ok(p.clone());
        }
        let mut b = Publisher::builder(full.clone());
        if let Some(ep) = &self.inner.endpoint {
            b = b.with_endpoint(ep.clone());
        }
        let publisher = b
            .build()
            .await
            .context("build google_cloud_pubsub Publisher")?;
        map.insert(full, publisher.clone());
        Ok(publisher)
    }
}
