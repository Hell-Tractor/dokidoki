use std::sync::Arc;

use axum::{
    extract::{
        State,
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::{api::extractors::AuthUser, state::AppState};

pub async fn handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state, user.id))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>, user_id: String) {
    let (connection_id, mut hub_rx) = state.ws_hub.register(user_id.clone()).await;
    tracing::info!(%user_id, connection_id, "ws connected");

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();

    let connected = serde_json::json!({
        "type": "connected",
        "payload": { "user_id": user_id },
    });
    let _ = out_tx.send(connected.to_string());

    let out_for_hub = out_tx.clone();
    let hub_forward = tokio::spawn(async move {
        while let Some(text) = hub_rx.recv().await {
            if out_for_hub.send(text).is_err() {
                break;
            }
        }
    });

    let (mut write, mut read) = socket.split();
    let write_task = tokio::spawn(async move {
        while let Some(text) = out_rx.recv().await {
            if write
                .send(Message::Text(text.into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    while let Some(result) = read.next().await {
        match result {
            Ok(Message::Text(text)) => {
                if let Some(reply) = handle_client_message(&state, connection_id, &user_id, &text).await {
                    if out_tx.send(reply).is_err() {
                        break;
                    }
                }
            }
            Ok(Message::Close(_)) => {
                tracing::debug!(%user_id, connection_id, "ws client closed");
                break;
            }
            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
            Ok(_) => {}
            Err(err) => {
                tracing::warn!(%user_id, connection_id, "ws socket error: {err}");
                break;
            }
        }
    }

    hub_forward.abort();
    write_task.abort();
    state.ws_hub.unregister(connection_id).await;
    tracing::info!(%user_id, connection_id, "ws disconnected");
}

async fn handle_client_message(
    state: &AppState,
    connection_id: u64,
    user_id: &str,
    text: &str,
) -> Option<String> {
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(err) => {
            tracing::debug!(
                %user_id,
                connection_id,
                "ws malformed frame ignored: {err}"
            );
            return None;
        }
    };
    let Some(msg_type) = value.get("type").and_then(|value| value.as_str()) else {
        tracing::debug!(%user_id, connection_id, "ws frame missing type ignored");
        return None;
    };
    match msg_type {
        "subscribe" => {
            let Some(conversation_id) = value
                .pointer("/payload/conversation_id")
                .and_then(|value| value.as_str())
            else {
                tracing::debug!(
                    %user_id,
                    connection_id,
                    "ws subscribe missing conversation_id ignored"
                );
                return None;
            };
            if state
                .ws_hub
                .subscribe(connection_id, conversation_id)
                .await
            {
                tracing::debug!(
                    %user_id,
                    connection_id,
                    conversation_id,
                    "ws subscribed"
                );
            }
            None
        }
        "ping" => Some(r#"{"type":"pong"}"#.to_owned()),
        other => {
            tracing::debug!(%user_id, connection_id, msg_type = other, "ws unknown type ignored");
            None
        }
    }
}
