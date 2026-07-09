use axum::http::StatusCode;
use serde_json::json;

use dokidoki_server::test_support::{
    http::{
        get, get_with_auth, patch_json_with_auth, post_json, register_body, unique_username,
    },
    setup_app,
};

#[tokio::test]
async fn get_me_without_token_returns_401() {
    let mut app = setup_app().await;
    let (status, body) = get(&mut app, "/api/v1/me").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "INVALID_TOKEN");
}

#[tokio::test]
async fn get_me_with_invalid_token_returns_401() {
    let mut app = setup_app().await;
    let (status, body) = get_with_auth(&mut app, "/api/v1/me", "doki_invalid").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "INVALID_TOKEN");
}

#[tokio::test]
async fn get_me_returns_current_user() {
    let mut app = setup_app().await;
    let username = unique_username("me");
    let password = "secret123";

    let (_, register_body) = post_json(
        &mut app,
        "/api/v1/auth/register",
        register_body(&username, password),
    )
    .await;
    let token = register_body["data"]["token"].as_str().unwrap();

    let (status, body) = get_with_auth(&mut app, "/api/v1/me", token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["username"], username);
    assert_eq!(body["data"]["display_name"], username);
    assert_eq!(body["data"]["max_proactive_per_day"], 20);
}

#[tokio::test]
async fn patch_me_updates_profile_fields() {
    let mut app = setup_app().await;
    let username = unique_username("patch");
    let password = "secret123";

    let (_, register_body) = post_json(
        &mut app,
        "/api/v1/auth/register",
        register_body(&username, password),
    )
    .await;
    let token = register_body["data"]["token"].as_str().unwrap();

    let (status, body) = patch_json_with_auth(
        &mut app,
        "/api/v1/me",
        token,
        json!({
            "display_name": "小明",
            "birthday": "2000-01-01",
            "max_proactive_per_day": 15,
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["username"], username);
    assert_eq!(body["data"]["display_name"], "小明");
    assert_eq!(body["data"]["birthday"], "2000-01-01");
    assert_eq!(body["data"]["max_proactive_per_day"], 15);
}

#[tokio::test]
async fn patch_me_with_empty_body_keeps_profile() {
    let mut app = setup_app().await;
    let username = unique_username("patch_empty");
    let password = "secret123";

    let (_, register_body) = post_json(
        &mut app,
        "/api/v1/auth/register",
        register_body(&username, password),
    )
    .await;
    let token = register_body["data"]["token"].as_str().unwrap();

    let (status, body) = patch_json_with_auth(&mut app, "/api/v1/me", token, json!({})).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["display_name"], username);
    assert_eq!(body["data"]["max_proactive_per_day"], 20);
}

#[tokio::test]
async fn patch_me_without_token_returns_401() {
    let mut app = setup_app().await;

    let (status, body) = patch_json_with_auth(
        &mut app,
        "/api/v1/me",
        "doki_invalid",
        json!({ "display_name": "小明" }),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "INVALID_TOKEN");
}

#[tokio::test]
async fn patch_me_invalid_display_name_returns_400() {
    let mut app = setup_app().await;
    let username = unique_username("patch_bad");
    let password = "secret123";

    let (_, register_body) = post_json(
        &mut app,
        "/api/v1/auth/register",
        register_body(&username, password),
    )
    .await;
    let token = register_body["data"]["token"].as_str().unwrap();

    let (status, body) = patch_json_with_auth(
        &mut app,
        "/api/v1/me",
        token,
        json!({ "display_name": "" }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "BAD_REQUEST");
}
