mod common;

use axum::http::StatusCode;
use serde_json::json;

use common::{
    assert_auth_success, assert_error, login_body, post_json, register_body, unique_username,
};

async fn setup() -> axum::Router {
    dokidoki_server::test_support::setup_app().await
}

#[tokio::test]
async fn register_returns_201_with_token_and_user() {
    let mut app = setup().await;
    let username = unique_username("alice");
    let password = "secret123";

    let (status, body) = post_json(
        &mut app,
        "/api/v1/auth/register",
        register_body(&username, password),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_auth_success(status, &body, &username);
}

#[tokio::test]
async fn register_with_display_name_and_birthday() {
    let mut app = setup().await;
    let username = unique_username("bob");

    let (status, body) = post_json(
        &mut app,
        "/api/v1/auth/register",
        json!({
            "username": username,
            "password": "secret123",
            "display_name": "小明",
            "birthday": "2000-01-01",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let data = &body["data"];
    assert_eq!(data["user"]["display_name"], "小明");
    assert_eq!(data["user"]["birthday"], "2000-01-01");
}

#[tokio::test]
async fn register_defaults_display_name_to_username() {
    let mut app = setup().await;
    let username = unique_username("carol");

    let (status, body) = post_json(
        &mut app,
        "/api/v1/auth/register",
        json!({
            "username": username,
            "password": "secret123",
            "display_name": "",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["user"]["display_name"], username);
}

#[tokio::test]
async fn register_duplicate_username_returns_409() {
    let mut app = setup().await;
    let username = unique_username("dup");
    let password = "secret123";

    let (first_status, _) = post_json(
        &mut app,
        "/api/v1/auth/register",
        register_body(&username, password),
    )
    .await;
    assert_eq!(first_status, StatusCode::CREATED);

    let (status, body) = post_json(
        &mut app,
        "/api/v1/auth/register",
        register_body(&username, password),
    )
    .await;

    assert_error(status, &body, StatusCode::CONFLICT, "USERNAME_TAKEN");
}

#[tokio::test]
async fn register_short_password_returns_400() {
    let mut app = setup().await;
    let username = unique_username("short_pw");

    let (status, body) = post_json(
        &mut app,
        "/api/v1/auth/register",
        register_body(&username, "short"),
    )
    .await;

    assert_error(status, &body, StatusCode::BAD_REQUEST, "BAD_REQUEST");
}

#[tokio::test]
async fn register_invalid_json_returns_400() {
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = setup().await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from("{not-json"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"]["code"], "BAD_REQUEST");
}

#[tokio::test]
async fn login_returns_200_after_register() {
    let mut app = setup().await;
    let username = unique_username("login_ok");
    let password = "secret123";

    let (register_status, _) = post_json(
        &mut app,
        "/api/v1/auth/register",
        register_body(&username, password),
    )
    .await;
    assert_eq!(register_status, StatusCode::CREATED);

    let (status, body) = post_json(
        &mut app,
        "/api/v1/auth/login",
        login_body(&username, password),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_auth_success(status, &body, &username);
}

#[tokio::test]
async fn login_wrong_password_returns_401() {
    let mut app = setup().await;
    let username = unique_username("login_bad_pw");
    let password = "secret123";

    post_json(
        &mut app,
        "/api/v1/auth/register",
        register_body(&username, password),
    )
    .await;

    let (status, body) = post_json(
        &mut app,
        "/api/v1/auth/login",
        login_body(&username, "wrongpass1"),
    )
    .await;

    assert_error(status, &body, StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS");
}

#[tokio::test]
async fn login_unknown_user_returns_401() {
    let mut app = setup().await;

    let (status, body) = post_json(
        &mut app,
        "/api/v1/auth/login",
        login_body("nobody", "secret123"),
    )
    .await;

    assert_error(status, &body, StatusCode::UNAUTHORIZED, "INVALID_CREDENTIALS");
}

#[tokio::test]
async fn login_short_password_returns_400() {
    let mut app = setup().await;
    let username = unique_username("login_short");

    post_json(
        &mut app,
        "/api/v1/auth/register",
        register_body(&username, "secret123"),
    )
    .await;

    let (status, body) = post_json(
        &mut app,
        "/api/v1/auth/login",
        login_body(&username, "short"),
    )
    .await;

    assert_error(status, &body, StatusCode::BAD_REQUEST, "BAD_REQUEST");
}

#[tokio::test]
async fn login_issues_new_token_each_time() {
    let mut app = setup().await;
    let username = unique_username("login_token");
    let password = "secret123";

    post_json(
        &mut app,
        "/api/v1/auth/register",
        register_body(&username, password),
    )
    .await;

    let (_, first) = post_json(
        &mut app,
        "/api/v1/auth/login",
        login_body(&username, password),
    )
    .await;
    let (_, second) = post_json(
        &mut app,
        "/api/v1/auth/login",
        login_body(&username, password),
    )
    .await;

    let first_token = first["data"]["token"].as_str().unwrap();
    let second_token = second["data"]["token"].as_str().unwrap();
    assert_ne!(first_token, second_token);
}
