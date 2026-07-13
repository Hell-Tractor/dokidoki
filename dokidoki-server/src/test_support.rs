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
    pub pool: MySqlPool,
    pub upload_dir: std::path::PathBuf,
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

/// 清空测试表并返回可 `oneshot` 的 Router。每个测试使用独立连接池，避免共享池耗尽。
pub async fn setup_app() -> TestApp {
    let guard = DB_TEST_LOCK.lock().expect("test db lock poisoned");

    let url = std::env::var("TEST_DATABASE_URL").expect(
        "TEST_DATABASE_URL is required for integration tests \
         (e.g. mysql://user:pass@127.0.0.1:3306/dokidoki_test)",
    );
    let mut config = config::Config::for_test(url);
    let upload_dir = std::env::temp_dir().join(format!("dokidoki-upload-{}", uuid::Uuid::new_v4()));
    config.upload.dir = upload_dir.to_string_lossy().into_owned();

    let pool = init_test_database(&config.database.url).await;
    reset_test_tables(&pool).await;

    let state = Arc::new(AppState::from_parts(config, pool.clone()));
    TestApp {
        app: api::router(state),
        pool,
        upload_dir,
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
    let pool = crate::db::pool::connect(url)
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

/// 返回一个可用于集成测试的连接池；每次调用都会创建新的连接池并跑迁移。
/// 不会重置表，适用于需要在测试中手动控制表状态的场景。
pub async fn test_pool() -> MySqlPool {
    let url = std::env::var("TEST_DATABASE_URL").expect(
        "TEST_DATABASE_URL is required for integration tests \
         (e.g. mysql://user:pass@127.0.0.1:3306/dokidoki_test)",
    );
    init_test_database(&url).await
}

pub async fn set_character_avatar_path(
    pool: &MySqlPool,
    character_id: &str,
    avatar_path: &str,
) {
    sqlx::query("UPDATE characters SET avatar_path = ? WHERE id = ?")
        .bind(avatar_path)
        .bind(character_id)
        .execute(pool)
        .await
        .expect("set character avatar_path");
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

    sqlx::query(
        r#"
        INSERT INTO character_states (character_id, current_activity, current_mood, availability)
        VALUES (?, '在线', '平静', 'high')
        "#,
    )
    .bind(&id)
    .execute(pool)
    .await
    .expect("insert test character state");

    id
}
