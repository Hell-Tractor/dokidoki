use crate::{
    db::queries::characters as character_queries,
    error::AppError,
    upload::{UploadStore, PLACEHOLDER_AVATAR},
};

pub struct AvatarImage {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
}

pub async fn character_avatar(
    pool: &sqlx::MySqlPool,
    upload: &UploadStore,
    character_id: &str,
) -> Result<AvatarImage, AppError> {
    let character = character_queries::find_by_id(pool, character_id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;

    if let Some(path) = character.avatar_path.filter(|p| !p.trim().is_empty()) {
        match upload.read(&path).await {
            Ok(bytes) => {
                return Ok(AvatarImage {
                    bytes,
                    content_type: UploadStore::content_type(&path),
                });
            }
            Err(err) if err.code() == crate::error::ErrorCode::NotFound => {}
            Err(err) => return Err(err),
        }
    }

    Ok(AvatarImage {
        bytes: PLACEHOLDER_AVATAR.to_vec(),
        content_type: "image/png",
    })
}
