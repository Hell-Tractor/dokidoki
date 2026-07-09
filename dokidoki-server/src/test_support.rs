//! 集成测试辅助；需要设置环境变量 `TEST_DATABASE_URL`。

pub mod http;

use std::sync::{Arc, Mutex};

use axum::Router;
use sqlx::MySqlPool;

use crate::{api, config, state::AppState};

static DB_TEST_LOCK: Mutex<()> = Mutex::new(());

/// 连接测试库、跑迁移、清空 auth 相关表，返回可 `oneshot` 的 Router。
pub async fn setup_app() -> Router {
    let _guard = DB_TEST_LOCK.lock().expect("test db lock poisoned");

    let url = std::env::var("TEST_DATABASE_URL").expect(
        "TEST_DATABASE_URL is required for integration tests \
         (e.g. mysql://user:pass@127.0.0.1:3306/dokidoki_test)",
    );

    let pool = init_test_database(&url).await;
    reset_auth_tables(&pool).await;

    let config = config::Config::for_test(url);
    let state = Arc::new(AppState::from_parts(config, pool));
    api::router(state)
}

async fn init_test_database(url: &str) -> MySqlPool {
    let pool = MySqlPool::connect(url)
        .await
        .expect("connect to TEST_DATABASE_URL");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations on test database");
    pool
}

pub async fn reset_auth_tables(pool: &MySqlPool) {
    sqlx::query("SET FOREIGN_KEY_CHECKS = 0")
        .execute(pool)
        .await
        .expect("disable foreign key checks");
    sqlx::query("TRUNCATE TABLE user_sessions")
        .execute(pool)
        .await
        .expect("truncate user_sessions");
    sqlx::query("TRUNCATE TABLE users")
        .execute(pool)
        .await
        .expect("truncate users");
    sqlx::query("SET FOREIGN_KEY_CHECKS = 1")
        .execute(pool)
        .await
        .expect("enable foreign key checks");
}
