use dokidoki_server::test_support::test_pool;

#[tokio::test]
async fn mysql_session_uses_utc() {
    let pool = test_pool().await;

    let (time_zone,): (String,) = sqlx::query_as("SELECT @@session.time_zone")
        .fetch_one(&pool)
        .await
        .expect("session time_zone");

    assert_eq!(time_zone, "+00:00");
}
