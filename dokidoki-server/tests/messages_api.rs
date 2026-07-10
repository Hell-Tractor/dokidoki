use axum::http::StatusCode;
use serde_json::json;

use dokidoki_server::test_support::{
    http::{
        assert_error, get_with_auth, post_json, post_json_with_auth, register_body, unique_username,
    },
    insert_test_character, setup_app,
};

async fn register_and_token(app: &mut dokidoki_server::test_support::TestApp) -> String {
    let username = unique_username("msg");
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
    let character_id = insert_test_character(&app.pool, "小咲").await;
    let (_, body) = post_json_with_auth(
        app,
        "/api/v1/conversations",
        token,
        json!({ "character_id": character_id }),
    )
    .await;
    body["data"]["id"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn list_messages_returns_empty_array() {
    let mut app = setup_app().await;
    let token = register_and_token(&mut app).await;
    let conversation_id = create_test_conversation(&mut app, &token).await;

    let (status, body) = get_with_auth(
        &mut app,
        &format!("/api/v1/conversations/{conversation_id}/messages"),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]["messages"].as_array().unwrap().is_empty());
    assert_eq!(body["data"]["has_more"], false);
}

#[tokio::test]
async fn create_message_returns_202() {
    let mut app = setup_app().await;
    let token = register_and_token(&mut app).await;
    let conversation_id = create_test_conversation(&mut app, &token).await;

    let (status, body) = post_json_with_auth(
        &mut app,
        &format!("/api/v1/conversations/{conversation_id}/messages"),
        &token,
        json!({ "content": "今天好累" }),
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(body["data"]["id"].as_str().is_some());
    assert!(body["data"]["turn_id"].as_str().is_some());
    assert!(body["data"]["created_at"].as_str().is_some());
}

#[tokio::test]
async fn list_messages_returns_messages_in_ascending_order() {
    let mut app = setup_app().await;
    let token = register_and_token(&mut app).await;
    let conversation_id = create_test_conversation(&mut app, &token).await;

    post_json_with_auth(
        &mut app,
        &format!("/api/v1/conversations/{conversation_id}/messages"),
        &token,
        json!({ "content": "第一条" }),
    )
    .await;
    post_json_with_auth(
        &mut app,
        &format!("/api/v1/conversations/{conversation_id}/messages"),
        &token,
        json!({ "content": "第二条" }),
    )
    .await;

    let (status, body) = get_with_auth(
        &mut app,
        &format!("/api/v1/conversations/{conversation_id}/messages"),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let messages = body["data"]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["content"], "第一条");
    assert_eq!(messages[1]["content"], "第二条");
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[0]["content_type"], "text");
    assert!(messages[0]["content"].is_string());
}

#[tokio::test]
async fn create_message_empty_content_returns_400() {
    let mut app = setup_app().await;
    let token = register_and_token(&mut app).await;
    let conversation_id = create_test_conversation(&mut app, &token).await;

    let (status, body) = post_json_with_auth(
        &mut app,
        &format!("/api/v1/conversations/{conversation_id}/messages"),
        &token,
        json!({ "content": "   " }),
    )
    .await;

    assert_error(status, &body, StatusCode::BAD_REQUEST, "BAD_REQUEST");
}

#[tokio::test]
async fn list_messages_unknown_conversation_returns_404() {
    let mut app = setup_app().await;
    let token = register_and_token(&mut app).await;

    let (status, body) = get_with_auth(
        &mut app,
        "/api/v1/conversations/00000000-0000-0000-0000-000000000000/messages",
        &token,
    )
    .await;

    assert_error(status, &body, StatusCode::NOT_FOUND, "NOT_FOUND");
}

#[tokio::test]
async fn list_messages_supports_before_pagination() {
    let mut app = setup_app().await;
    let token = register_and_token(&mut app).await;
    let conversation_id = create_test_conversation(&mut app, &token).await;

    for content in ["一", "二", "三"] {
        post_json_with_auth(
            &mut app,
            &format!("/api/v1/conversations/{conversation_id}/messages"),
            &token,
            json!({ "content": content }),
        )
        .await;
    }

    let (status, body) = get_with_auth(
        &mut app,
        &format!("/api/v1/conversations/{conversation_id}/messages?limit=2"),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let messages = body["data"]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["content"], "二");
    assert_eq!(messages[1]["content"], "三");
    assert_eq!(body["data"]["has_more"], true);

    let before_id = messages[0]["id"].as_str().unwrap();
    let (status, body) = get_with_auth(
        &mut app,
        &format!(
            "/api/v1/conversations/{conversation_id}/messages?before={before_id}&limit=2"
        ),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let messages = body["data"]["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["content"], "一");
    assert_eq!(body["data"]["has_more"], false);
}
