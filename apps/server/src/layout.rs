//! The administration screen's layout: which pages exist and which widgets sit
//! on them. The Server only stores it; the Web decides what it means, so it
//! is kept as JSON with a few limits and a revision that counts saves.
//!
//! Stored in a single file (`DECKOX_LAYOUT_FILE`). Every browser also keeps its
//! own copy and prefers it; the revision lets a browser tell that the Server
//! has a newer layout than the one it last saved.

use std::{env, path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::fsutil::atomic_write_secure;

const DEFAULT_LAYOUT_FILE: &str = "/var/lib/deckox/layout.json";
const MAX_LAYOUT_BYTES: usize = 256 * 1024;
const MAX_PAGES: usize = 50;

#[derive(Clone)]
pub struct LayoutStore {
    path: PathBuf,
    write_lock: Arc<Mutex<()>>,
}

/// What is on disk, and what `GET /api/v1/layout` answers.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StoredLayout {
    /// Counts saves; `0` while nothing has been saved.
    pub revision: u64,
    /// `None` until a layout is saved, when the Web uses its recommended one.
    pub layout: Option<Value>,
}

impl LayoutStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            write_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn from_env() -> Self {
        Self::new(PathBuf::from(
            env::var("DECKOX_LAYOUT_FILE").unwrap_or_else(|_| DEFAULT_LAYOUT_FILE.to_owned()),
        ))
    }

    pub async fn get(&self) -> StoredLayout {
        let Ok(content) = tokio::fs::read_to_string(&self.path).await else {
            return StoredLayout::default();
        };
        serde_json::from_str(&content).unwrap_or_else(|error| {
            tracing::warn!(%error, "layout file is not valid; using none");
            StoredLayout::default()
        })
    }

    /// Saves `layout` and returns the new revision.
    pub async fn put(&self, layout: Value) -> Result<StoredLayout, String> {
        let pages = validate(&layout)?;
        let _guard = self.write_lock.lock().await;
        let stored = StoredLayout {
            revision: self.get().await.revision + 1,
            layout: Some(layout),
        };
        let body = serde_json::to_vec(&stored).map_err(|error| error.to_string())?;
        atomic_write_secure(&self.path, &body).await?;
        tracing::debug!(pages, revision = stored.revision, "layout saved");
        Ok(stored)
    }
}

/// Checks the shape the Web relies on and returns the page count.
fn validate(layout: &Value) -> Result<usize, String> {
    let object = layout.as_object().ok_or("layout must be a JSON object")?;
    if object.get("version").and_then(Value::as_u64).is_none() {
        return Err("layout needs a numeric version".to_owned());
    }
    let pages = object
        .get("pages")
        .and_then(Value::as_array)
        .ok_or("layout needs a pages list")?;
    if pages.len() > MAX_PAGES {
        return Err(format!("layout has more than {MAX_PAGES} pages"));
    }
    let size = serde_json::to_vec(layout).map_or(usize::MAX, |bytes| bytes.len());
    if size > MAX_LAYOUT_BYTES {
        return Err(format!("layout is larger than {MAX_LAYOUT_BYTES} bytes"));
    }
    Ok(pages.len())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::LayoutStore;

    fn store() -> LayoutStore {
        LayoutStore::new(std::env::temp_dir().join(format!(
            "deckox-layout-test-{}.json",
            hex::encode(rand::random::<[u8; 8]>())
        )))
    }

    #[tokio::test]
    async fn starts_empty_and_counts_saves() {
        let store = store();
        let empty = store.get().await;
        assert_eq!(empty.revision, 0);
        assert!(empty.layout.is_none());

        let first = store
            .put(json!({"version": 1, "pages": []}))
            .await
            .expect("valid layout");
        assert_eq!(first.revision, 1);
        let second = store
            .put(json!({"version": 1, "pages": [{"id": "a"}]}))
            .await
            .expect("valid layout");
        assert_eq!(second.revision, 2);

        let read = store.get().await;
        assert_eq!(read.revision, 2);
        assert_eq!(read.layout, second.layout);
        let _ = tokio::fs::remove_file(&store.path).await;
    }

    #[tokio::test]
    async fn refuses_layouts_the_web_cannot_read() {
        let store = store();
        for bad in [
            json!([]),
            json!({"pages": []}),
            json!({"version": "1", "pages": []}),
            json!({"version": 1}),
            json!({"version": 1, "pages": vec![json!({}); 51]}),
            json!({"version": 1, "pages": [], "pad": "x".repeat(300_000)}),
        ] {
            assert!(store.put(bad).await.is_err());
        }
        assert_eq!(
            store.get().await.revision,
            0,
            "a refused save changes nothing"
        );
    }

    #[tokio::test]
    async fn treats_a_damaged_file_as_no_layout() {
        let store = store();
        tokio::fs::write(&store.path, b"{not json")
            .await
            .expect("write");
        assert!(store.get().await.layout.is_none());
        let _ = tokio::fs::remove_file(&store.path).await;
    }
}
