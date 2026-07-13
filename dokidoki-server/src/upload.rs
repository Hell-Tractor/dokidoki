use std::path::{Component, Path, PathBuf};

use crate::error::AppError;

pub const PLACEHOLDER_AVATAR: &[u8] = include_bytes!("../assets/avatars/default.png");

pub struct UploadStore {
    root: PathBuf,
}

impl UploadStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.root.join("avatars"))
    }

    pub fn resolve(&self, relative: &str) -> Result<PathBuf, AppError> {
        let path = Path::new(relative);
        if path.is_absolute() {
            return Err(AppError::internal(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "absolute upload path rejected",
            )));
        }
        if path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(AppError::internal(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "parent dir in upload path rejected",
            )));
        }
        Ok(self.root.join(path))
    }

    pub async fn read(&self, relative: &str) -> Result<Vec<u8>, AppError> {
        let full = self.resolve(relative)?;
        tokio::fs::read(&full).await.map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                AppError::not_found("文件不存在")
            } else {
                AppError::internal(err)
            }
        })
    }

    pub fn content_type(path: &str) -> &'static str {
        match Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("webp") => "image/webp",
            _ => "application/octet-stream",
        }
    }

    /// 将仓库内 `assets/avatars/` 同步到 `{upload.dir}/avatars/`（已存在则跳过）。
    pub fn bootstrap_avatars(&self) -> std::io::Result<()> {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/avatars");
        if !source.is_dir() {
            return Ok(());
        }

        self.ensure_dirs()?;
        let target_root = self.root.join("avatars");

        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let file_name = entry.file_name();
            let target = target_root.join(&file_name);
            if target.exists() {
                continue;
            }
            std::fs::copy(entry.path(), target)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_rejects_path_traversal() {
        let store = UploadStore::new("/tmp/uploads");
        assert!(store.resolve("../etc/passwd").is_err());
        assert!(store.resolve("avatars/../secret.png").is_err());
    }

    #[test]
    fn content_type_from_extension() {
        assert_eq!(UploadStore::content_type("avatars/a.png"), "image/png");
        assert_eq!(UploadStore::content_type("avatars/a.JPG"), "image/jpeg");
    }
}
