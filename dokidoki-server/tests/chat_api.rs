use std::time::Duration;

use axum::http::StatusCode;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::time::sleep;

use dokidoki_server::test_support::{
    http::{
        get_with_auth, post_json, post_json_with_auth, register_body, unique_username,
    },
    insert_test_character, setup_app,
};

async fn register_and_token(app: &mut dokidoki_server::test_support::TestApp) -> String {
    let username = unique_username("chat");
    let (_, body) = post_json(
        app,
        "/api/v1/auth/register",
        register_body(&username, "secret123"),
    )
    .await;
    body["data"]["token"].as_str().unwrap().to_owned()
}

async fn create_test_conversation(
    app: &mut dokidoki_server::test_support::TestApp,
    token: &str,
) -> String {
    post_json(
        app,
        "/api/v1/dev/llm/queue",
        json!({ "responses": ["[REPLY]"] }),
    )
    .await;

    let character_id = insert_test_character(&app.pool, "小咲").await;
    let (_, body) = post_json_with_auth(
        app,
        "/api/v1/conversations",
        token,
        json!({ "character_id": character_id }),
    )
    .await;

    sleep(Duration::from_millis(150)).await;
    body["data"]["id"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn user_message_triggers_fake_llm_character_reply() {
    let mut app = setup_app().await;
    let token = register_and_token(&mut app).await;
    let conversation_id = create_test_conversation(&mut app, &token).await;

    post_json(
        &mut app,
        "/api/v1/dev/llm/queue",
        json!({ "responses": ["[REPLY] 怎么了？"] }),
    )
    .await;

    post_json_with_auth(
        &mut app,
        &format!("/api/v1/conversations/{conversation_id}/messages"),
        &token,
        json!({ "content": "在吗" }),
    )
    .await;

    sleep(Duration::from_millis(200)).await;

    let (status, body) = get_with_auth(
        &mut app,
        &format!("/api/v1/conversations/{conversation_id}/messages"),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let messages = body["data"]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[0]["content"], "在吗");
    assert_eq!(messages[1]["role"], "character");
    assert_eq!(messages[1]["content"], "怎么了？");
}

#[tokio::test]
async fn fake_llm_splits_multiple_bubbles() {
    let mut app = setup_app().await;
    let token = register_and_token(&mut app).await;
    let conversation_id = create_test_conversation(&mut app, &token).await;

    post_json(
        &mut app,
        "/api/v1/dev/llm/queue",
        json!({ "responses": ["[REPLY] 第一句|||第二句"] }),
    )
    .await;

    post_json_with_auth(
        &mut app,
        &format!("/api/v1/conversations/{conversation_id}/messages"),
        &token,
        json!({ "content": "你好" }),
    )
    .await;

    sleep(Duration::from_millis(200)).await;

    let (_, body) = get_with_auth(
        &mut app,
        &format!("/api/v1/conversations/{conversation_id}/messages"),
        &token,
    )
    .await;

    let messages = body["data"]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[1]["content"], "第一句");
    assert_eq!(messages[2]["content"], "第二句");
    assert_eq!(messages[1]["seq_in_turn"], 0);
    assert_eq!(messages[2]["seq_in_turn"], 1);
}

#[tokio::test]
async fn ws_receives_character_reply_after_subscribe() {
    use axum::http::header::AUTHORIZATION;
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{client::IntoClientRequest, Message},
    };

    let mut app = setup_app().await;
    let token = register_and_token(&mut app).await;
    let conversation_id = create_test_conversation(&mut app, &token).await;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind ws test port");
    let addr = listener.local_addr().expect("local addr");
    let router = app.clone();
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve ws test");
    });

    let mut request = format!("ws://{addr}/api/v1/ws")
        .into_client_request()
        .expect("ws request");
    request.headers_mut().insert(
        AUTHORIZATION,
        format!("Bearer {token}").parse().expect("auth header"),
    );

    let (mut ws, _) = connect_async(request).await.expect("ws connect");
    let connected = ws.next().await.expect("connected frame").expect("connected ok");
    let connected: serde_json::Value =
        serde_json::from_str(connected.to_text().expect("connected text")).expect("connected json");
    assert_eq!(connected["type"], "connected");

    ws.send(Message::Text(
        json!({
            "type": "subscribe",
            "payload": { "conversation_id": conversation_id },
        })
        .to_string()
        .into(),
    ))
    .await
    .expect("subscribe");

    post_json(
        &mut app,
        "/api/v1/dev/llm/queue",
        json!({ "responses": ["[REPLY] WS 收到了"] }),
    )
    .await;

    post_json_with_auth(
        &mut app,
        &format!("/api/v1/conversations/{conversation_id}/messages"),
        &token,
        json!({ "content": "ping" }),
    )
    .await;

    let mut saw_message = false;
    for _ in 0..20 {
        if let Some(Ok(Message::Text(text))) =
            tokio::time::timeout(Duration::from_millis(200), ws.next()).await.ok().flatten()
        {
            let event: serde_json::Value = serde_json::from_str(&text).expect("event json");
            if event["type"] == "message" {
                assert_eq!(event["payload"]["role"], "character");
                assert_eq!(event["payload"]["content"], "WS 收到了");
                assert_eq!(event["payload"]["conversation_id"], conversation_id);
                saw_message = true;
                break;
            }
        }
    }
    assert!(saw_message, "expected character message over ws");
}

#[tokio::test]
async fn dev_llm_queue_empty_responses_returns_400() {
    let mut app = setup_app().await;

    let (status, body) = post_json(
        &mut app,
        "/api/v1/dev/llm/queue",
        json!({ "responses": [] }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "BAD_REQUEST");
}

#[tokio::test]
async fn ws_ping_returns_pong() {
    use axum::http::header::AUTHORIZATION;
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{client::IntoClientRequest, Message},
    };

    let mut app = setup_app().await;
    let token = register_and_token(&mut app).await;

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind ws test port");
    let addr = listener.local_addr().expect("local addr");
    let router = app.clone();
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve ws test");
    });

    let mut request = format!("ws://{addr}/api/v1/ws")
        .into_client_request()
        .expect("ws request");
    request.headers_mut().insert(
        AUTHORIZATION,
        format!("Bearer {token}").parse().expect("auth header"),
    );

    let (mut ws, _) = connect_async(request).await.expect("ws connect");
    let _ = ws.next().await;

    ws.send(Message::Text(json!({ "type": "ping" }).to_string().into()))
        .await
        .expect("ping");

    let pong = ws.next().await.expect("pong frame").expect("pong ok");
    let pong: serde_json::Value =
        serde_json::from_str(pong.to_text().expect("pong text")).expect("pong json");
    assert_eq!(pong["type"], "pong");
}
