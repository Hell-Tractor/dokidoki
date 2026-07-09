use axum::http::StatusCode;

use dokidoki_server::test_support::{
    http::{get, get_with_auth, post_json, register_body, unique_username},
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
