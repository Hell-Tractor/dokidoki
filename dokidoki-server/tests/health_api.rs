use axum::http::StatusCode;

use dokidoki_server::test_support::{http::get, setup_app_without_db};

#[tokio::test]
async fn health_returns_200_ok() {
    let mut app = setup_app_without_db();
    let (status, body) = get(&mut app, "/api/v1/health").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"], "ok");
}
