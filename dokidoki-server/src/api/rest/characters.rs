use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use chrono::NaiveTime;
use serde::{Deserialize, Deserializer, Serialize};
use validator::Validate;

use crate::{
    api::{extractors::AuthUser, response::ApiResponse, response::ApiResult, ValidatedJson},
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
    #[serde(default, deserialize_with = "deserialize_patch_string")]
    dnd_start: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_patch_string")]
    dnd_end: Option<Option<String>>,
}

/// 字段缺失 → `None`（不更新）；JSON `null` → `Some(None)`（清空）；字符串 → `Some(Some(_))`（设置）。
fn deserialize_patch_string<'de, D>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = Option<Option<String>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("null or HH:MM string")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(Some(None))
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(Some(None))
        }

        fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
            Ok(Some(Some(value.to_owned())))
        }

        fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
            Ok(Some(Some(value)))
        }
    }

    deserializer.deserialize_any(Visitor)
}

impl Validate for UpdateCharacterSettingsRequest {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        let mut errors = validator::ValidationErrors::new();
        for (field, value) in [("dnd_start", &self.dnd_start), ("dnd_end", &self.dnd_end)] {
            if let Some(Some(time)) = value {
                if parse_wall_clock(time).is_err() {
                    errors.add(field, validator::ValidationError::new("invalid_wall_clock"));
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl From<UpdateCharacterSettingsRequest> for UpdateCharacterSettingsInput {
    fn from(body: UpdateCharacterSettingsRequest) -> Self {
        Self {
            dnd_start: map_patch_field(body.dnd_start),
            dnd_end: map_patch_field(body.dnd_end),
        }
    }
}

fn map_patch_field(field: Option<Option<String>>) -> PatchField<NaiveTime> {
    match field {
        None => PatchField::Unset,
        Some(None) => PatchField::Clear,
        Some(Some(value)) => PatchField::Set(
            parse_wall_clock(&value).expect("validated before domain mapping"),
        ),
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
    ValidatedJson(body): ValidatedJson<UpdateCharacterSettingsRequest>,
) -> ApiResult<CharacterSettingsResponse> {
    let settings = character_settings::update_for_user(
        &state.db,
        &user.id,
        &character_id,
        body.into(),
    )
    .await?;
    Ok(ApiResponse::ok(settings.into()))
}
