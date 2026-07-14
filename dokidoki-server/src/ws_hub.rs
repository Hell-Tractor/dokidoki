use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use tokio::sync::{Mutex, mpsc};

pub struct WsHub {
    next_id: AtomicU64,
    connections: Mutex<HashMap<u64, WsConnection>>,
}

struct WsConnection {
    user_id: String,
    subscriptions: HashSet<String>,
    tx: mpsc::UnboundedSender<String>,
}

impl WsHub {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            connections: Mutex::new(HashMap::new()),
        }
    }

    pub async fn register(&self, user_id: String) -> (u64, mpsc::UnboundedReceiver<String>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.connections.lock().await.insert(
            id,
            WsConnection {
                user_id,
                subscriptions: HashSet::new(),
                tx,
            },
        );
        (id, rx)
    }

    pub async fn unregister(&self, connection_id: u64) {
        self.connections.lock().await.remove(&connection_id);
    }

    pub async fn subscribe(&self, connection_id: u64, conversation_id: &str) -> bool {
        let mut connections = self.connections.lock().await;
        if let Some(connection) = connections.get_mut(&connection_id) {
            connection.subscriptions.insert(conversation_id.to_owned());
            true
        } else {
            tracing::debug!(
                connection_id,
                conversation_id,
                "ws subscribe ignored: connection gone"
            );
            false
        }
    }

    pub async fn emit_json(
        &self,
        user_id: &str,
        conversation_id: &str,
        event_type: &str,
        payload: impl Serialize,
    ) {
        let envelope = serde_json::json!({
            "type": event_type,
            "payload": payload,
        });
        let text = match serde_json::to_string(&envelope) {
            Ok(text) => text,
            Err(err) => {
                tracing::error!("ws envelope serialize failed: {err}");
                return;
            }
        };

        let connections = self.connections.lock().await;
        let mut sent = 0u32;
        let mut dropped = 0u32;
        for connection in connections.values() {
            if connection.user_id == user_id
                && connection.subscriptions.contains(conversation_id)
            {
                if connection.tx.send(text.clone()).is_err() {
                    dropped += 1;
                } else {
                    sent += 1;
                }
            }
        }
        if dropped > 0 {
            tracing::warn!(
                user_id,
                conversation_id,
                event_type,
                dropped,
                sent,
                "ws emit dropped: subscriber channel closed"
            );
        }
    }
}
