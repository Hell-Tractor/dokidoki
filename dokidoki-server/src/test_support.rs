//! 集成测试辅助；需要设置环境变量 `TEST_DATABASE_URL`。

pub mod http;

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use axum::Router;
use sqlx::MySqlPool;
use tokio::sync::OnceCell;

use crate::{api, config, state::AppState};

static DB_TEST_LOCK: Mutex<()> = Mutex::new(());
static SHARED_POOL: OnceCell<MySqlPool> = OnceCell::const_new();

pub struct TestApp {
    pub app: Router,
    pub pool: MySqlPool,
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

/// 清空测试表并返回可 `oneshot` 的 Router。连接池与迁移在进程内只初始化一次。
pub async fn setup_app() -> TestApp {
    let guard = DB_TEST_LOCK.lock().expect("test db lock poisoned");

    let pool = shared_test_pool().await;
    reset_test_tables(&pool).await;

    let url = std::env::var("TEST_DATABASE_URL").expect(
        "TEST_DATABASE_URL is required for integration tests \
         (e.g. mysql://user:pass@127.0.0.1:3306/dokidoki_test)",
    );
    let config = config::Config::for_test(url);
    let state = Arc::new(AppState::from_parts(config, pool.clone()));
    TestApp {
        app: api::router(state),
        pool,
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

async fn shared_test_pool() -> MySqlPool {
    SHARED_POOL
        .get_or_init(|| async {
            let url = std::env::var("TEST_DATABASE_URL").expect(
                "TEST_DATABASE_URL is required for integration tests \
                 (e.g. mysql://user:pass@127.0.0.1:3306/dokidoki_test)",
            );
            init_test_database(&url).await
        })
        .await
        .clone()
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

/// 清空集成测试涉及的表，保证用例之间隔离。
pub async fn reset_test_tables(pool: &MySqlPool) {
    sqlx::raw_sql(
        "\
        SET FOREIGN_KEY_CHECKS = 0;
        TRUNCATE TABLE messages;
        TRUNCATE TABLE conversations;
        TRUNCATE TABLE proactive_logs;
        TRUNCATE TABLE user_memories;
        TRUNCATE TABLE user_character_settings;
        TRUNCATE TABLE character_states;
        TRUNCATE TABLE characters;
        TRUNCATE TABLE user_sessions;
        TRUNCATE TABLE users;
        SET FOREIGN_KEY_CHECKS = 1;
        ",
    )
    .execute(pool)
    .await
    .expect("reset test tables");
}

pub async fn test_pool() -> MySqlPool {
    shared_test_pool().await
}

pub async fn insert_test_character(pool: &MySqlPool, name: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO characters (id, name, persona_json, schedule_json)
        VALUES (?, ?, '{}', '{}')
        "#,
    )
    .bind(&id)
    .bind(name)
    .execute(pool)
    .await
    .expect("insert test character");
    id
}
