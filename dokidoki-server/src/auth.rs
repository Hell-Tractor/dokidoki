use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString},
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

pub struct RegisterParams {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
    pub birthday: Option<NaiveDate>,
}

pub struct AuthSession {
    pub token: String,
    pub user: User,
}

pub async fn register(
    pool: &MySqlPool,
    auth_config: &Auth,
    params: RegisterParams,
) -> Result<AuthSession, AppError> {
    let display_name = params
        .display_name
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| params.username.clone());

    let password_hash = hash_password(&params.password, auth_config.password_cost)?;
    let user_id = Uuid::new_v4().to_string();
    let session_id = Uuid::new_v4().to_string();
    let token = generate_token(&auth_config.token_prefix);
    let token_hash = hash_token(&token);

    let mut tx = pool.begin().await.map_err(AppError::internal)?;

    let user = queries::users::insert(
        &mut *tx,
        &user_id,
        &params.username,
        &password_hash,
        &display_name,
        params.birthday,
    )
    .await?;

    queries::sessions::insert(&mut *tx, &session_id, &user_id, &token_hash).await?;

    tx.commit().await.map_err(AppError::internal)?;

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

fn generate_token(prefix: &str) -> String {
    let random = Uuid::new_v4().simple().to_string();
    format!("{prefix}{random}")
}

fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
