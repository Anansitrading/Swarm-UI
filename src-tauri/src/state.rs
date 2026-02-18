use portable_pty::MasterPty;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use tantivy::merge_policy::LogMergePolicy;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy};

use crate::search::schema::IndexSchema;
use crate::sprites_api::SpritesClient;
use crate::sprites_ws::WsState;

/// Represents a single PTY instance
pub struct PtyInstance {
    pub id: String,
    pub master: Box<dyn MasterPty + Send>,
    pub writer: Box<dyn Write + Send>,
    pub child: Box<dyn portable_pty::Child + Send>,
    pub cols: u16,
    pub rows: u16,
}

/// Handle to the Tantivy search index, shared across watcher and query threads.
pub struct IndexHandle {
    pub index: Index,
    pub reader: IndexReader,
    pub schema: IndexSchema,
    pub writer: Arc<Mutex<IndexWriter>>,
    pub paused: Arc<AtomicBool>,
}

impl IndexHandle {
    /// Create a new IndexHandle from an existing Tantivy `Index`.
    ///
    /// - Sets `LogMergePolicy` on the writer immediately after creation.
    /// - Creates a reader with `ReloadPolicy::OnCommitWithDelay`.
    /// - `heap_bytes`: writer buffer size (512MB for bulk, 50MB for watcher).
    pub fn new(index: Index, schema: IndexSchema, heap_bytes: usize) -> tantivy::Result<Self> {
        let writer: IndexWriter = index.writer(heap_bytes)?;
        writer.set_merge_policy(Box::new(LogMergePolicy::default()));

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            schema,
            writer: Arc::new(Mutex::new(writer)),
            paused: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Get a fresh `Searcher` from the reader.
    pub fn searcher(&self) -> tantivy::Searcher {
        self.reader.searcher()
    }
}

/// Shared application state wrapped in Mutex for thread safety
pub struct AppState {
    pub ptys: Mutex<HashMap<String, PtyInstance>>,
    pub sprites_client: Mutex<Option<SpritesClient>>,
    pub ws_state: WsState,
    /// Tantivy search index handle. Initialized lazily on first use or app startup.
    pub index_handle: Mutex<Option<IndexHandle>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            ptys: Mutex::new(HashMap::new()),
            sprites_client: Mutex::new(None),
            ws_state: WsState::new(),
            index_handle: Mutex::new(None),
        }
    }

    /// Update the sprites client when settings change
    pub fn set_sprites_client(&self, base_url: String, token: String) {
        let client = SpritesClient::new(base_url, token);
        *self.sprites_client.lock().unwrap() = Some(client);
    }

    /// Get a reference to the sprites client, returning error if not configured
    pub fn get_sprites_client(&self) -> Result<SpritesClient, crate::error::AppError> {
        let guard = self.sprites_client.lock().unwrap();
        match &*guard {
            Some(client) => {
                // Clone the client data to create a new one (SpritesClient is cheap to recreate)
                Ok(SpritesClient::new(
                    client.base_url().to_string(),
                    client.token().to_string(),
                ))
            }
            None => Err(crate::error::AppError::Internal(
                "Sprites API not configured. Go to Settings to enter your API token.".to_string(),
            )),
        }
    }
}

/// PTY spawn configuration from frontend
#[derive(Debug, Deserialize)]
pub struct PtySpawnConfig {
    pub shell: Option<String>,
    pub args: Option<Vec<String>>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
}

/// PTY info returned to frontend
#[derive(Debug, Serialize, Clone)]
pub struct PtyInfo {
    pub id: String,
    pub pid: u32,
    pub cols: u16,
    pub rows: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;
    use tantivy::Index;

    fn create_test_index_handle(heap_bytes: usize) -> IndexHandle {
        let schema = IndexSchema::new();
        let index = Index::create_in_ram(schema.schema.clone());
        IndexHandle::new(index, schema, heap_bytes).expect("failed to create IndexHandle")
    }

    #[test]
    fn test_index_handle_new_creates_valid_handle() {
        let handle = create_test_index_handle(50_000_000);

        // Writer should be accessible (lock succeeds)
        let _writer = handle.writer.lock().unwrap();
        drop(_writer);

        // Paused should start as false
        assert!(!handle.paused.load(Ordering::Relaxed));
    }

    #[test]
    fn test_index_handle_searcher_returns_searcher() {
        let handle = create_test_index_handle(50_000_000);
        let searcher = handle.searcher();
        // Empty index has 1 segment reader (the empty one)
        assert_eq!(searcher.num_docs(), 0);
    }

    #[test]
    fn test_index_handle_writer_is_shared() {
        let handle = create_test_index_handle(50_000_000);
        let writer_clone = Arc::clone(&handle.writer);

        // Both references should point to the same writer
        let _guard1 = handle.writer.lock().unwrap();
        assert!(writer_clone.try_lock().is_err(), "should be locked by first ref");
    }

    #[test]
    fn test_index_handle_paused_is_shared() {
        let handle = create_test_index_handle(50_000_000);
        let paused_clone = Arc::clone(&handle.paused);

        handle.paused.store(true, Ordering::Relaxed);
        assert!(paused_clone.load(Ordering::Relaxed));
    }

    #[test]
    fn test_app_state_new_has_no_index_handle() {
        let state = AppState::new();
        let guard = state.index_handle.lock().unwrap();
        assert!(guard.is_none());
    }
}
