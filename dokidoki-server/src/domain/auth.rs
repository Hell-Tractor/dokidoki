use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use chrono::NaiveDate;
use rand_core::OsRng;
use sha2::{Digest, Sha256};
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    config::Auth,
    db::{models::User, queries},
    error::AppError,
};

pub struct RegisterInput {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
    pub birthday: Option<NaiveDate>,
}

pub struct LoginInput {
    pub username: String,
    pub password: String,
}

pub struct AuthSession {
    pub token: String,
    pub user: User,
}

pub async fn register(
    pool: &MySqlPool,
    auth_config: &Auth,
    input: RegisterInput,
) -> Result<AuthSession, AppError> {
    let display_name = input
        .display_name
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| input.username.clone());

    let password_hash = hash_password(&input.password, auth_config.password_cost)?;
    let user_id = Uuid::new_v4().to_string();

    let mut tx = pool.begin().await.map_err(AppError::internal)?;

    let user = queries::users::insert(
        &mut tx,
        &user_id,
        &input.username,
        &password_hash,
        &display_name,
        input.birthday,
    )
    .await?;

    let session = create_session(&mut *tx, auth_config, user).await?;

    tx.commit().await.map_err(AppError::internal)?;

    Ok(session)
}

pub async fn login(
    pool: &MySqlPool,
    auth_config: &Auth,
    input: LoginInput,
) -> Result<AuthSession, AppError> {
    let credentials = queries::users::find_by_username(pool, &input.username)
        .await?
        .ok_or_else(AppError::invalid_credentials)?;

    verify_password(&input.password, &credentials.password_hash)?;

    let user = credentials.into();
    create_session(pool, auth_config, user).await
}

async fn create_session<'e, E>(
    executor: E,
    auth_config: &Auth,
    user: User,
) -> Result<AuthSession, AppError>
where
    E: sqlx::Executor<'e, Database = sqlx::MySql>,
{
    let session_id = Uuid::new_v4().to_string();
    let token = generate_token(&auth_config.token_prefix);
    let token_hash = hash_token(&token);

    queries::sessions::insert(executor, &session_id, &user.id, &token_hash).await?;

    Ok(AuthSession { token, user })
}

fn hash_password(password: &str, password_cost: u32) -> Result<String, AppError> {
    let params = argon2::Params::new(19456, password_cost, 1, None).map_err(|e| {
        tracing::error!("argon2 params: {e:?}");
        AppError::with_code(crate::error::ErrorCode::InternalError)
    })?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon2.hash_password(password.as_bytes(), &salt).map_err(|e| {
        tracing::error!("argon2 hash: {e:?}");
        AppError::with_code(crate::error::ErrorCode::InternalError)
    })?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, password_hash: &str) -> Result<(), AppError> {
    let parsed = PasswordHash::new(password_hash).map_err(|e| {
        tracing::error!("invalid password hash in db: {e:?}");
        AppError::with_code(crate::error::ErrorCode::InternalError)
    })?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| AppError::invalid_credentials())
}

fn generate_token(prefix: &str) -> String {
    let random = Uuid::new_v4().simple().to_string();
    format!("{prefix}{random}")
}

fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub async fn authenticate(pool: &MySqlPool, token: &str) -> Result<User, AppError> {
    let token_hash = hash_token(token);
    let user_id = queries::sessions::find_user_id_by_token_hash(pool, &token_hash)
        .await?
        .ok_or_else(AppError::invalid_token)?;
    queries::users::find_by_id(pool, &user_id)
        .await?
        .ok_or_else(AppError::invalid_token)
}
