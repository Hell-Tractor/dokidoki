use axum::http::StatusCode;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

use dokidoki_server::test_support::{
    http::{
        get_with_auth, post_json, post_json_with_auth, register_body, unique_username,
    },
    insert_test_character, setup_app,
};

async fn register_and_token(app: &mut dokidoki_server::test_support::TestApp) -> String {
    let username = unique_username("conv");
    let (_, body) = post_json(
        app,
        "/api/v1/auth/register",
        register_body(&username, "secret123"),
    )
    .await;
    body["data"]["token"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn list_characters_returns_empty_array() {
    let mut app = setup_app().await;
    let token = register_and_token(&mut app).await;

    let (status, body) = get_with_auth(&mut app, "/api/v1/characters", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn list_characters_returns_characters() {
    let mut app = setup_app().await;
    let character_id = insert_test_character(&app.pool, "小咲").await;
    let token = register_and_token(&mut app).await;

    let (status, body) = get_with_auth(&mut app, "/api/v1/characters", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"][0]["id"], character_id);
    assert_eq!(body["data"][0]["name"], "小咲");
    assert_eq!(
        body["data"][0]["avatar_url"],
        format!("/api/v1/characters/{character_id}/avatar")
    );
}

#[tokio::test]
async fn create_conversation_returns_201() {
    let mut app = setup_app().await;
    let character_id = insert_test_character(&app.pool, "凛").await;
    let token = register_and_token(&mut app).await;

    let (status, body) = post_json_with_auth(
        &mut app,
        "/api/v1/conversations",
        &token,
        json!({ "character_id": character_id }),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["character_id"], character_id);
    assert_eq!(body["data"]["status"], "active");
    assert_eq!(body["data"]["first_contact_done"], false);
}

#[tokio::test]
async fn create_conversation_is_idempotent() {
    let mut app = setup_app().await;
    let character_id = insert_test_character(&app.pool, "凛").await;
    let token = register_and_token(&mut app).await;

    let (first_status, first_body) = post_json_with_auth(
        &mut app,
        "/api/v1/conversations",
        &token,
        json!({ "character_id": character_id }),
    )
    .await;
    assert_eq!(first_status, StatusCode::CREATED);
    let conversation_id = first_body["data"]["id"].as_str().unwrap();

    let (second_status, second_body) = post_json_with_auth(
        &mut app,
        "/api/v1/conversations",
        &token,
        json!({ "character_id": character_id }),
    )
    .await;

    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(second_body["data"]["id"], conversation_id);
}

#[tokio::test]
async fn create_conversation_unknown_character_returns_404() {
    let mut app = setup_app().await;
    let token = register_and_token(&mut app).await;

    let (status, body) = post_json_with_auth(
        &mut app,
        "/api/v1/conversations",
        &token,
        json!({ "character_id": "00000000-0000-0000-0000-000000000000" }),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn list_conversations_returns_user_conversations() {
    let mut app = setup_app().await;
    let character_id = insert_test_character(&app.pool, "小咲").await;
    let token = register_and_token(&mut app).await;

    post_json_with_auth(
        &mut app,
        "/api/v1/conversations",
        &token,
        json!({ "character_id": character_id }),
    )
    .await;

    let (status, body) = get_with_auth(&mut app, "/api/v1/conversations", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"][0]["character_name"], "小咲");
    assert!(body["data"][0]["last_message"].is_null());
    assert!(body["data"][0]["current_activity"].is_null());
}

#[tokio::test]
async fn create_conversation_triggers_icebreaker_messages() {
    let mut app = setup_app().await;
    let character_id = insert_test_character(&app.pool, "小爱").await;
    let token = register_and_token(&mut app).await;

    post_json(
        &mut app,
        "/api/v1/dev/llm/queue",
        json!({ "responses": ["[REPLY] 嗨|||你终于来了"] }),
    )
    .await;

    let (status, body) = post_json_with_auth(
        &mut app,
        "/api/v1/conversations",
        &token,
        json!({ "character_id": character_id }),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let conversation_id = body["data"]["id"].as_str().unwrap();

    sleep(Duration::from_millis(300)).await;

    let (_, messages_body) = get_with_auth(
        &mut app,
        &format!("/api/v1/conversations/{conversation_id}/messages"),
        &token,
    )
    .await;

    let messages = messages_body["data"]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["role"], "character");
    assert_eq!(messages[0]["content"], "嗨");
    assert_eq!(messages[1]["content"], "你终于来了");

    let (_, existing_body) = post_json_with_auth(
        &mut app,
        "/api/v1/conversations",
        &token,
        json!({ "character_id": character_id }),
    )
    .await;

    assert_eq!(existing_body["data"]["first_contact_done"], true);
}
