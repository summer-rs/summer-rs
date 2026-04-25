use bytes::Bytes;
use google_cloud_pubsub::subscriber::handler::Handler;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Incoming Pub/Sub payload plus shared acknowledgement state
#[derive(Clone)]
pub struct Message {
    pub message_id: String,
    pub data: Bytes,
    pub attributes: HashMap<String, String>,
    ack: Arc<Mutex<Option<Handler>>>,
}

impl Message {
    pub(crate) fn new(
        message_id: String,
        data: Bytes,
        attributes: HashMap<String, String>,
        ack: Arc<Mutex<Option<Handler>>>,
    ) -> Self {
        Self {
            message_id,
            data,
            attributes,
            ack,
        }
    }

    /// Acknowledge this message (at-least-once best-effort semantics from the client library).
    pub fn ack(&self) {
        if let Ok(mut slot) = self.ack.lock() {
            if let Some(h) = slot.take() {
                h.ack();
            }
        }
    }

    /// Negative acknowledgement: the message may be redelivered.
    pub fn nack(&self) {
        if let Ok(mut slot) = self.ack.lock() {
            if let Some(h) = slot.take() {
                h.nack();
            }
        }
    }
}

impl Drop for Message {
    fn drop(&mut self) {
        if let Ok(mut slot) = self.ack.lock() {
            if let Some(h) = slot.take() {
                h.ack();
            }
        }
    }
}
