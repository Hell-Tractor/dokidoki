use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Serialize;

use crate::{
    api::{extractors::AuthUser, response::ApiResponse, response::ApiResult},
    domain::{avatar, characters},
    db::models::Character,
    state::AppState,
};

pub fn api() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_characters))
        .route("/{id}/avatar", get(get_avatar))
}

#[derive(Serialize)]
struct CharacterResponse {
    id: String,
    name: String,
    avatar_url: String,
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

struct AvatarResponse {
    content_type: &'static str,
    bytes: Vec<u8>,
}

impl IntoResponse for AvatarResponse {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static(self.content_type),
        );
        headers.insert(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_static("private, max-age=3600"),
        );
        (StatusCode::OK, headers, self.bytes).into_response()
    }
}

async fn list_characters(
    State(state): State<Arc<AppState>>,
    AuthUser(_user): AuthUser,
) -> ApiResult<Vec<CharacterResponse>> {
    let characters = characters::list_all(&state.db).await?;
    Ok(ApiResponse::ok(
        characters.into_iter().map(CharacterResponse::from).collect(),
    ))
}

async fn get_avatar(
    State(state): State<Arc<AppState>>,
    AuthUser(_user): AuthUser,
    Path(character_id): Path<String>,
) -> Result<AvatarResponse, crate::error::AppError> {
    let image = avatar::character_avatar(&state.db, &state.upload, &character_id).await?;
    Ok(AvatarResponse {
        content_type: image.content_type,
        bytes: image.bytes,
    })
}
