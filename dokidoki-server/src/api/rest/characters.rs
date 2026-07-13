use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use chrono::NaiveTime;
use serde::{Deserialize, Serialize};

use crate::{
    api::{extractors::AuthUser, response::ApiResponse, response::ApiResult},
    domain::{
        avatar,
        character_settings::{
            self, parse_wall_clock, CharacterSettings, PatchField, UpdateCharacterSettingsInput,
        },
        characters,
    },
    db::models::Character,
    state::AppState,
};

pub fn api() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_characters))
        .route("/{id}/avatar", get(get_avatar))
        .route(
            "/{id}/settings",
            get(get_settings).put(update_settings),
        )
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

#[derive(Serialize)]
struct CharacterSettingsResponse {
    dnd_start: Option<String>,
    dnd_end: Option<String>,
    push_muted: bool,
}

impl From<CharacterSettings> for CharacterSettingsResponse {
    fn from(settings: CharacterSettings) -> Self {
        Self {
            dnd_start: settings.dnd_start.map(format_wall_clock),
            dnd_end: settings.dnd_end.map(format_wall_clock),
            push_muted: settings.push_muted,
        }
    }
}

fn format_wall_clock(time: NaiveTime) -> String {
    time.format("%H:%M").to_string()
}

#[derive(Deserialize, Default)]
struct UpdateCharacterSettingsRequest {
    #[serde(default)]
    dnd_start: Option<NullableField>,
    #[serde(default)]
    dnd_end: Option<NullableField>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NullableField {
    Null,
    Value(String),
}

impl UpdateCharacterSettingsRequest {
    fn into_input(self) -> Result<UpdateCharacterSettingsInput, crate::error::AppError> {
        Ok(UpdateCharacterSettingsInput {
            dnd_start: map_nullable_field(self.dnd_start)?,
            dnd_end: map_nullable_field(self.dnd_end)?,
        })
    }
}

fn map_nullable_field(
    field: Option<NullableField>,
) -> Result<PatchField<NaiveTime>, crate::error::AppError> {
    match field {
        None => Ok(PatchField::Unset),
        Some(NullableField::Null) => Ok(PatchField::Clear),
        Some(NullableField::Value(value)) => Ok(PatchField::Set(parse_wall_clock(&value)?)),
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

async fn get_settings(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(character_id): Path<String>,
) -> ApiResult<CharacterSettingsResponse> {
    let settings =
        character_settings::get_for_user(&state.db, &user.id, &character_id).await?;
    Ok(ApiResponse::ok(settings.into()))
}

async fn update_settings(
    State(state): State<Arc<AppState>>,
    AuthUser(user): AuthUser,
    Path(character_id): Path<String>,
    axum::Json(body): axum::Json<UpdateCharacterSettingsRequest>,
) -> ApiResult<CharacterSettingsResponse> {
    let settings = character_settings::update_for_user(
        &state.db,
        &user.id,
        &character_id,
        body.into_input()?,
    )
    .await?;
    Ok(ApiResponse::ok(settings.into()))
}
