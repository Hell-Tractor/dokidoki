use std::fs;

use axum::http::StatusCode;

use dokidoki_server::{
    test_support::{
        http::{get_bytes, get_bytes_with_auth, post_json, register_body, unique_username},
        insert_test_character, set_character_avatar_path, setup_app,
    },
    upload::PLACEHOLDER_AVATAR,
};

async fn register_and_token(app: &mut dokidoki_server::test_support::TestApp) -> String {
    let username = unique_username("avatar");
    let (_, body) = post_json(
        app,
        "/api/v1/auth/register",
        register_body(&username, "secret123"),
    )
    .await;
    body["data"]["token"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn get_avatar_returns_placeholder_when_no_file() {
    let mut app = setup_app().await;
    let character_id = insert_test_character(&app.pool, "小咲").await;
    let token = register_and_token(&mut app).await;

    let (status, bytes, content_type) = get_bytes_with_auth(
        &mut app,
        &format!("/api/v1/characters/{character_id}/avatar"),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("image/png"));
    assert_eq!(bytes, PLACEHOLDER_AVATAR);
}

#[tokio::test]
async fn get_avatar_returns_stored_image() {
    let mut app = setup_app().await;
    let character_id = insert_test_character(&app.pool, "小咲").await;
    let token = register_and_token(&mut app).await;

    let avatar_path = format!("avatars/{character_id}.png");
    let full_path = app.upload_dir.join(&avatar_path);
    fs::create_dir_all(full_path.parent().unwrap()).expect("create avatars dir");
    fs::write(&full_path, b"fake-png-bytes").expect("write avatar");
    set_character_avatar_path(&app.pool, &character_id, &avatar_path).await;

    let (status, bytes, content_type) = get_bytes_with_auth(
        &mut app,
        &format!("/api/v1/characters/{character_id}/avatar"),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("image/png"));
    assert_eq!(bytes, b"fake-png-bytes");
}

#[tokio::test]
async fn get_avatar_unknown_character_returns_404() {
    let mut app = setup_app().await;
    let token = register_and_token(&mut app).await;

    let (status, _, _) = get_bytes_with_auth(
        &mut app,
        "/api/v1/characters/not-a-real-id/avatar",
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_avatar_requires_auth() {
    let mut app = setup_app().await;
    let character_id = insert_test_character(&app.pool, "小咲").await;

    let (status, _, _) = get_bytes(
        &mut app,
        &format!("/api/v1/characters/{character_id}/avatar"),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
