use std::path::Path;

use tokio::io::AsyncWriteExt;

/// Writes `contents` to `path` atomically: a `0600` temporary file is created
/// alongside `path`, written, `fsync`ed, then renamed over the target. Used
/// for every secret-bearing file Deckox owns (admin account, audit log) so a
/// crash mid-write never leaves a torn or partially-written file in place.
pub async fn atomic_write_secure(path: &Path, contents: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "path has no parent directory".to_owned())?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "path has an invalid file name".to_owned())?;
    let temporary_path = parent.join(format!(
        ".{file_name}.{}",
        hex::encode(rand::random::<[u8; 8]>())
    ));

    let mut options = tokio::fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);

    let result = async {
        let mut file = options
            .open(&temporary_path)
            .await
            .map_err(|error| format!("failed to create {}: {error}", temporary_path.display()))?;
        file.write_all(contents)
            .await
            .map_err(|error| format!("failed to write {}: {error}", temporary_path.display()))?;
        file.sync_all()
            .await
            .map_err(|error| format!("failed to sync {}: {error}", temporary_path.display()))?;
        drop(file);
        tokio::fs::rename(&temporary_path, path)
            .await
            .map_err(|error| format!("failed to replace {}: {error}", path.display()))
    }
    .await;

    if result.is_err() {
        let _ = tokio::fs::remove_file(&temporary_path).await;
    }
    result
}
