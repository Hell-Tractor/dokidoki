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
                if let Some(reply) = handle_client_message(&state, connection_id, &text).await {
                    if out_tx.send(reply).is_err() {
                        break;
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
            Ok(_) => {}
            Err(_) => break,
        }
    }

    hub_forward.abort();
    write_task.abort();
    state.ws_hub.unregister(connection_id).await;
}

async fn handle_client_message(
    state: &AppState,
    connection_id: u64,
    text: &str,
) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    match value.get("type").and_then(|value| value.as_str())? {
        "subscribe" => {
            let conversation_id = value
                .pointer("/payload/conversation_id")
                .and_then(|value| value.as_str())?;
            state
                .ws_hub
                .subscribe(connection_id, conversation_id)
                .await;
            None
        }
        "ping" => Some(r#"{"type":"pong"}"#.to_owned()),
        _ => None,
    }
}
