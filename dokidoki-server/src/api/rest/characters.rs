use std::sync::Arc;

use axum::{routing::get, Router};
use serde::Serialize;

use crate::{
    api::{response::ApiResponse, response::ApiResult},
    db::{models::Character, queries::characters},
    state::AppState,
};

pub fn api() -> Router<Arc<AppState>> {
    Router::new().route("/", get(list_characters))
}

#[derive(Serialize)]
pub struct CharacterResponse {
    pub id: String,
    pub name: String,
    pub avatar_url: String,
}

impl From<Character> for CharacterResponse {
    fn from(character: Character) -> Self {
        Self {
            avatar_url: format!("/api/v1/characters/{}/avatar", character.id),
            id: character.id,
            name: character.name,
        }
    }
}

async fn list_characters(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> ApiResult<Vec<CharacterResponse>> {
    let characters = characters::list_all(&state.db).await?;
    Ok(ApiResponse::ok(
        characters.into_iter().map(CharacterResponse::from).collect(),
    ))
}
