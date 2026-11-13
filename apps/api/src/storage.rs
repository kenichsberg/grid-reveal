//! Picture storage.
//!
//! S3-compatible object storage in production (Cloudflare R2), or local disk
//! for development so the API runs without object storage credentials.

use std::path::{Path, PathBuf};

use s3::creds::Credentials;
use s3::{Bucket, Region};

use crate::config::StorageConfig;
use crate::error::{AppError, AppResult};

pub enum Storage {
    S3 {
        bucket: Box<Bucket>,
        /// Set when the bucket is served from a public domain; otherwise the
        /// API streams the bytes itself.
        public_base_url: Option<String>,
    },
    Local {
        dir: PathBuf,
    },
}

impl Storage {
    pub fn from_config(config: &StorageConfig) -> AppResult<Self> {
        match config {
            StorageConfig::S3 {
                endpoint,
                bucket,
                access_key_id,
                secret_access_key,
                public_base_url,
            } => {
                let region = Region::Custom {
                    region: "auto".to_string(),
                    endpoint: endpoint.clone(),
                };
                let credentials = Credentials::new(
                    Some(access_key_id),
                    Some(secret_access_key),
                    None,
                    None,
                    None,
                )
                .map_err(|e| AppError::internal(format!("invalid storage credentials: {e}")))?;

                // R2's account endpoint addresses buckets by path.
                let bucket = Bucket::new(bucket, region, credentials)
                    .map_err(|e| AppError::internal(format!("storage setup failed: {e}")))?
                    .with_path_style();

                Ok(Storage::S3 {
                    bucket,
                    public_base_url: public_base_url.clone(),
                })
            }
            StorageConfig::Local { dir } => {
                let dir = PathBuf::from(dir);
                std::fs::create_dir_all(&dir).map_err(|e| {
                    AppError::internal(format!("could not create {}: {e}", dir.display()))
                })?;
                tracing::warn!(
                    dir = %dir.display(),
                    "using local picture storage; configure R2_* for anything but development"
                );
                Ok(Storage::Local { dir })
            }
        }
    }

    pub async fn put(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> AppResult<()> {
        match self {
            Storage::S3 { bucket, .. } => {
                bucket
                    .put_object_with_content_type(key, &bytes, content_type)
                    .await
                    .map_err(|e| AppError::internal(format!("upload failed: {e}")))?;
                Ok(())
            }
            Storage::Local { dir } => {
                let path = self.local_path(dir, key)?;
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(|e| AppError::internal(format!("write failed: {e}")))?;
                }
                tokio::fs::write(&path, &bytes)
                    .await
                    .map_err(|e| AppError::internal(format!("write failed: {e}")))?;
                Ok(())
            }
        }
    }

    pub async fn get(&self, key: &str) -> AppResult<Vec<u8>> {
        match self {
            Storage::S3 { bucket, .. } => {
                let response = bucket
                    .get_object(key)
                    .await
                    .map_err(|_| AppError::NotFound)?;
                Ok(response.to_vec())
            }
            Storage::Local { dir } => {
                let path = self.local_path(dir, key)?;
                tokio::fs::read(&path).await.map_err(|_| AppError::NotFound)
            }
        }
    }

    pub async fn delete(&self, key: &str) -> AppResult<()> {
        match self {
            Storage::S3 { bucket, .. } => {
                bucket
                    .delete_object(key)
                    .await
                    .map_err(|e| AppError::internal(format!("delete failed: {e}")))?;
                Ok(())
            }
            Storage::Local { dir } => {
                let path = self.local_path(dir, key)?;
                match tokio::fs::remove_file(&path).await {
                    Ok(()) => Ok(()),
                    // Already gone is the desired end state.
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    Err(e) => Err(AppError::internal(format!("delete failed: {e}"))),
                }
            }
        }
    }

    /// Where a client should fetch this picture.
    ///
    /// Absolute when the bucket has a public domain, otherwise a path relative
    /// to the API that the client joins with its configured base URL.
    pub fn public_url(&self, key: &str) -> String {
        match self {
            Storage::S3 {
                public_base_url: Some(base),
                ..
            } => format!("{}/{}", base.trim_end_matches('/'), key),
            _ => format!("/pictures/raw/{key}"),
        }
    }

    /// Resolves a storage key under the local directory, refusing anything that
    /// would escape it.
    fn local_path(&self, dir: &Path, key: &str) -> AppResult<PathBuf> {
        if key.is_empty()
            || key.contains("..")
            || key.starts_with('/')
            || key.contains('\\')
            || !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'))
        {
            return Err(AppError::bad_request("invalid storage key"));
        }
        Ok(dir.join(key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local() -> Storage {
        Storage::Local {
            dir: PathBuf::from("/tmp/grid-reveal-test"),
        }
    }

    #[test]
    fn traversal_keys_are_refused() {
        let s = local();
        let dir = PathBuf::from("/tmp/grid-reveal-test");
        for bad in ["../secret", "/etc/passwd", "a/../../b", "", "a\\b", "a;b"] {
            assert!(s.local_path(&dir, bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn ordinary_keys_resolve_under_the_directory() {
        let s = local();
        let dir = PathBuf::from("/tmp/grid-reveal-test");
        let path = s.local_path(&dir, "pictures/abc123.webp").unwrap();
        assert!(path.starts_with(&dir));
    }

    #[test]
    fn public_url_uses_the_api_route_without_a_public_bucket() {
        assert_eq!(local().public_url("abc.webp"), "/pictures/raw/abc.webp");
    }
}
