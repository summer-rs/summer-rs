//! [![summer-rs](https://img.shields.io/github/stars/summer-rs/summer-rs)](https://summer-rs.github.io/)
#![doc(html_favicon_url = "https://summer-rs.github.io/favicon.ico")]
#![doc(html_logo_url = "https://summer-rs.github.io/logo.svg")]

pub mod config;
pub mod consumer;
pub mod extractor;
pub mod handler;
pub mod message;
pub mod producer;

pub use config::PubSubConfig;
pub use consumer::{Consumer, ConsumerOpts, Consumers, resolve_subscription, resolve_topic};
pub use google_cloud_pubsub;
pub use handler::{TypedConsumer, TypedHandlerRegistrar, auto_consumers};
pub use message::Message;
pub use producer::PubSubProducer;
pub use summer_macros::pubsub_listener;

use config::credentials_from_file;
use google_cloud_pubsub::client::Subscriber;
use summer::async_trait;
use summer::config::ConfigRegistry;
use summer::plugin::component::ComponentRef;
use summer::plugin::{ComponentRegistry, MutableComponentRegistry};
use summer::{
    app::{App, AppBuilder},
    plugin::Plugin,
};
use std::ops::Deref;
use std::sync::Arc;

pub use google_cloud_pubsub::*;

pub trait PubSubConfigurator {
    fn add_consumer(&mut self, consumers: Consumers) -> &mut Self;
}

impl PubSubConfigurator for AppBuilder {
    fn add_consumer(&mut self, new_consumers: Consumers) -> &mut Self {
        if let Some(consumers) = self.get_component_ref::<Consumers>() {
            unsafe {
                let raw_ptr = ComponentRef::into_raw(consumers);
                let consumers = &mut *(raw_ptr as *mut Consumers);
                consumers.merge(new_consumers);
            }
            self
        } else {
            self.add_component(new_consumers)
        }
    }
}

pub struct PubSubPlugin;

#[async_trait]
impl Plugin for PubSubPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let config = app
            .get_config::<PubSubConfig>()
            .expect("summer-pubsub: config with prefix `pubsub` is required");

        if !config.enabled {
            tracing::info!("summer-pubsub: disabled by config (`pubsub.enabled = false`)");
            return;
        }

        let creds_opt = config
            .credentials
            .as_ref()
            .map(|path| credentials_from_file(path))
            .transpose()
            .expect("summer-pubsub: load credentials failed");

        let mut sub_builder = Subscriber::builder();
        if let Some(endpoint) = &config.endpoint {
            sub_builder = sub_builder.with_endpoint(endpoint.clone());
        }
        if let Some(ref creds) = creds_opt {
            sub_builder = sub_builder.with_credentials(creds.clone());
        }

        let subscriber = sub_builder
            .build()
            .await
            .expect("summer-pubsub: create Subscriber failed");

        let producer = PubSubProducer::new(
            config.project_id.clone(),
            config.endpoint.clone(),
            creds_opt,
        );
        app.add_component(subscriber.clone());
        app.add_component(producer);

        if let Some(consumers) = app.get_component_ref::<Consumers>() {
            for consumer in consumers.deref().iter() {
                let instance = consumer.new_instance(config.project_id.as_str());
                app.add_scheduler(|app: Arc<App>| Box::new(instance.schedule(app)));
                tracing::info!(
                    "summer-pubsub: register scheduler for subscription `{}`",
                    consumer.subscription_literal
                );
            }
        } else {
            tracing::info!("summer-pubsub: no Consumers registered");
        }
    }
}
