use sqlx::MySqlPool;

use crate::{
    db::{models::Character, queries::characters as character_queries},
    error::AppError,
};

pub async fn list_all(pool: &MySqlPool) -> Result<Vec<Character>, AppError> {
    character_queries::list_all(pool).await
}
