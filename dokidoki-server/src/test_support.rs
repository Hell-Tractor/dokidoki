//! 集成测试辅助；需要设置环境变量 `TEST_DATABASE_URL`。

pub mod http;

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use axum::Router;
use sqlx::MySqlPool;

use crate::{api, config, state::AppState};

static DB_TEST_LOCK: Mutex<()> = Mutex::new(());

pub struct TestApp {
    pub app: Router,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl Deref for TestApp {
    type Target = Router;

    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

impl DerefMut for TestApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.app
    }
}

/// 连接测试库、跑迁移、清空 auth 相关表，返回可 `oneshot` 的 Router。
pub async fn setup_app() -> TestApp {
    let guard = DB_TEST_LOCK.lock().expect("test db lock poisoned");

    let url = std::env::var("TEST_DATABASE_URL").expect(
        "TEST_DATABASE_URL is required for integration tests \
         (e.g. mysql://user:pass@127.0.0.1:3306/dokidoki_test)",
    );

    let pool = init_test_database(&url).await;
    reset_auth_tables(&pool).await;

    let config = config::Config::for_test(url);
    let state = Arc::new(AppState::from_parts(config, pool));
    TestApp {
        app: api::router(state),
        _guard: guard,
    }
}

/// 不连接数据库的 Router；适用于 `/health` 等无 DB 依赖的测试。
pub fn setup_app_without_db() -> Router {
    let config = config::Config::for_test("mysql://127.0.0.1:3306/unused");
    let pool = MySqlPool::connect_lazy(&config.database.url).expect("lazy pool");
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
