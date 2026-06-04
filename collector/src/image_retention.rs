use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail};
use chrono::{DateTime, Duration, Utc};

use crate::storage::{ScreenshotStoreKind, Store, StoredScreenshotFile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageRetentionPolicy {
    pub retention_days: u32,
}

impl Default for ImageRetentionPolicy {
    fn default() -> Self {
        Self { retention_days: 30 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRetentionCleanupResult {
    pub deleted_files: usize,
    pub deleted_bytes: u64,
    pub failed_files: usize,
}

pub fn cleanup_expired_images(
    store: &mut Store,
    screenshot_dir: &Path,
    high_res_screenshot_dir: &Path,
    now: DateTime<Utc>,
    policy: ImageRetentionPolicy,
) -> Result<ImageRetentionCleanupResult> {
    let cutoff = now - Duration::days(policy.retention_days as i64);
    let files = store.list_expirable_screenshot_files(cutoff)?;
    let mut result = ImageRetentionCleanupResult {
        deleted_files: 0,
        deleted_bytes: 0,
        failed_files: 0,
    };

    for file in files {
        let base_dir = match file.kind {
            ScreenshotStoreKind::Thumbnail => screenshot_dir,
            ScreenshotStoreKind::HighRes => high_res_screenshot_dir,
        };
        match delete_screenshot_file(base_dir, &file) {
            Ok(()) => {
                store.mark_screenshot_file_expired(file.kind, file.id, now)?;
                result.deleted_files += 1;
                result.deleted_bytes += file.file_size_bytes;
            }
            Err(err) => {
                eprintln!(
                    "image retention cleanup failed for {}: {err:#}",
                    file.file_path
                );
                result.failed_files += 1;
            }
        }
    }

    Ok(result)
}

fn delete_screenshot_file(base_dir: &Path, file: &StoredScreenshotFile) -> Result<()> {
    let path = safe_screenshot_path(base_dir, &file.file_path)?;
    if !path.exists() {
        return Ok(());
    }
    std::fs::remove_file(path)?;
    Ok(())
}

fn safe_screenshot_path(base_dir: &Path, file_path: &str) -> Result<PathBuf> {
    let relative = Path::new(file_path);
    if relative.is_absolute() {
        bail!("screenshot path must be relative");
    }

    for component in relative.components() {
        match component {
            Component::Normal(_) => {}
            _ => bail!("screenshot path contains unsafe components"),
        }
    }

    Ok(base_dir.join(relative))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_path_rejects_parent_segments() {
        let err = safe_screenshot_path(Path::new("data/screenshots"), "../secret.jpg")
            .expect_err("parent segments must be rejected");
        assert!(err.to_string().contains("unsafe"));
    }
}
