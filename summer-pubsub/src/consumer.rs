use crate::handler::{BoxedHandler, HandlerArgs, PubSubEnvelope};
use google_cloud_pubsub::client::Subscriber;
use summer::app::App;
use summer::error::Result;
use summer::plugin::ComponentRegistry;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct Consumers(Vec<Consumer>);

impl Consumers {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_consumer(mut self, consumer: Consumer) -> Self {
        self.0.push(consumer);
        self
    }

    pub(crate) fn merge(&mut self, consumers: Self) {
        for consumer in consumers.0 {
            self.0.push(consumer);
        }
    }
}

impl Deref for Consumers {
    type Target = Vec<Consumer>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct Consumer {
    pub(crate) subscription_literal: &'static str,
    pub(crate) handler: BoxedHandler,
}

#[derive(Clone, Default)]
pub struct ConsumerOpts;

impl Consumer {
    pub fn default() -> ConsumerOpts {
        ConsumerOpts
    }

    pub(crate) fn new_instance(&self, project_id: &str) -> ConsumerInstance {
        ConsumerInstance {
            subscription: resolve_subscription(project_id, self.subscription_literal),
            handler: self.handler.clone(),
        }
    }
}

impl ConsumerOpts {
    pub fn consume<H, A>(self, subscription: &'static str, handler: H) -> Consumer
    where
        H: HandlerArgs<A> + Sync,
        A: 'static,
    {
        Consumer {
            handler: BoxedHandler::from_handler(handler),
            subscription_literal: subscription,
        }
    }
}

pub(crate) struct ConsumerInstance {
    subscription: String,
    handler: BoxedHandler,
}

impl ConsumerInstance {
    pub async fn schedule(self, app: Arc<App>) -> Result<String> {
        let ConsumerInstance {
            subscription,
            handler,
        } = self;
        let subscriber = app.get_component::<Subscriber>().expect(
            "summer-pubsub: Subscriber component missing; add PubSubPlugin before consumers run",
        );
        let mut stream = subscriber.subscribe(subscription.as_str()).build();
        loop {
            let next = stream.next().await;
            let Some(result) = next else {
                tracing::warn!("pubsub subscription {subscription}: stream closed");
                break;
            };
            let (grpc, h) = match result {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(?e, "pubsub subscription {subscription}: stream error");
                    break;
                }
            };
            let env = PubSubEnvelope::new(h);
            BoxedHandler::call(handler.clone(), grpc, env, app.clone()).await;
        }
        Ok(format!("pubsub consumer {subscription} stopped"))
    }
}

pub fn resolve_subscription(project_id: &str, literal: &str) -> String {
    if literal.starts_with("projects/") && literal.contains("/subscriptions/") {
        literal.to_string()
    } else {
        format!("projects/{project_id}/subscriptions/{literal}")
    }
}

pub fn resolve_topic(project_id: &str, literal: &str) -> String {
    if literal.starts_with("projects/") && literal.contains("/topics/") {
        literal.to_string()
    } else {
        format!("projects/{project_id}/topics/{literal}")
    }
}
