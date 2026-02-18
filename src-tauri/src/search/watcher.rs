use crate::search::doc_ext::DocExt;
use crate::search::indexer::{parse_jsonl_to_documents, SessionIndexEntry};
use crate::search::schema::IndexSchema;
use crate::search::types::SessionListItem;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tantivy::schema::Term;
use tantivy::{IndexReader, IndexWriter, TantivyDocument};
use tracing::{debug, warn};

/// Sessions-index.json file structure (local to watcher).
#[derive(Deserialize)]
struct SessionsIndexFile {
    entries: Vec<SessionIndexEntry>,
}

/// Convert a tantivy DateTime to an RFC 3339 string.
fn format_tantivy_date(dt: tantivy::DateTime) -> String {
    let secs = dt.into_timestamp_secs();
    chrono::DateTime::from_timestamp(secs, 0)
        .map(|d| d.to_rfc3339())
        .unwrap_or_default()
}

/// Convert a session TantivyDocument to a SessionListItem.
fn session_doc_to_list_item(doc: &TantivyDocument, schema: &IndexSchema) -> SessionListItem {
    SessionListItem {
        session_id: doc.get_str(schema.session_id).unwrap_or("").to_string(),
        project_path: doc.get_str(schema.project_path).unwrap_or("").to_string(),
        summary: doc.get_str(schema.summary).unwrap_or("").to_string(),
        first_prompt: doc.get_str(schema.first_prompt).unwrap_or("").to_string(),
        git_branch: doc.get_str(schema.git_branch).unwrap_or("").to_string(),
        model: doc.get_str(schema.model).unwrap_or("").to_string(),
        status: doc.get_str(schema.status).unwrap_or("").to_string(),
        message_count: doc.get_u64_val(schema.message_count).unwrap_or(0),
        total_tokens: doc.get_u64_val(schema.total_tokens).unwrap_or(0),
        created_at: doc.get_date_val(schema.created_at).map(format_tantivy_date),
        modified_at: doc.get_date_val(schema.modified_at).map(format_tantivy_date),
        has_tool_use: doc.get_bool_val(schema.has_tool_use).unwrap_or(false),
        file_exists: doc.get_bool_val(schema.file_exists).unwrap_or(true),
        archived: doc.get_bool_val(schema.archived).unwrap_or(false),
    }
}

/// Load session metadata from the sessions-index.json in the same directory as the JSONL file.
fn load_session_meta(jsonl_path: &Path) -> Option<SessionIndexEntry> {
    let parent = jsonl_path.parent()?;
    let index_path = parent.join("sessions-index.json");
    let content = std::fs::read_to_string(&index_path).ok()?;
    let index_file: SessionsIndexFile = serde_json::from_str(&content).ok()?;
    let session_id = jsonl_path.file_stem()?.to_str()?;
    index_file
        .entries
        .into_iter()
        .find(|e| e.session_id == session_id)
}

/// Find the session document for a given session_id via the index reader.
fn find_session_doc(
    session_id: &str,
    reader: &IndexReader,
    schema: &IndexSchema,
) -> Option<TantivyDocument> {
    let searcher = reader.searcher();
    let query = tantivy::query::BooleanQuery::new(vec![
        (
            tantivy::query::Occur::Must,
            Box::new(tantivy::query::TermQuery::new(
                Term::from_field_text(schema.session_id, session_id),
                tantivy::schema::IndexRecordOption::Basic,
            )),
        ),
        (
            tantivy::query::Occur::Must,
            Box::new(tantivy::query::TermQuery::new(
                Term::from_field_text(schema.doc_type, "session"),
                tantivy::schema::IndexRecordOption::Basic,
            )),
        ),
    ]);
    let top_docs = searcher
        .search(&query, &tantivy::collector::TopDocs::with_limit(1))
        .ok()?;
    let (_, doc_address) = top_docs.first()?;
    searcher.doc(*doc_address).ok()
}

/// Emit a `session:updated` event with a full SessionListItem payload.
fn emit_session_updated(app_handle: &Option<tauri::AppHandle>, item: &SessionListItem) {
    if let Some(handle) = app_handle {
        use tauri::Emitter;
        let _ = handle.emit("session:updated", item);
    }
}

/// Re-index a single session from its JSONL file.
///
/// Deletes all existing docs for the session, re-parses the file,
/// and adds the new documents. Returns the updated SessionListItem.
fn reindex_session(
    jsonl_path: &Path,
    writer: &Arc<Mutex<IndexWriter>>,
    schema: &IndexSchema,
) -> Option<SessionListItem> {
    let meta = load_session_meta(jsonl_path);
    let docs = parse_jsonl_to_documents(jsonl_path, schema, meta.as_ref());
    if docs.is_empty() {
        return None;
    }

    let item = session_doc_to_list_item(&docs[0], schema);
    let session_id = &item.session_id;

    let mut w = writer.lock().ok()?;
    w.delete_term(Term::from_field_text(schema.session_id, session_id));
    for doc in docs {
        if let Err(e) = w.add_document(doc) {
            warn!("Failed to add document: {e}");
            return None;
        }
    }
    if let Err(e) = w.commit() {
        warn!("Failed to commit after reindex: {e}");
        return None;
    }
    drop(w);

    debug!("Reindexed session {session_id}");
    Some(item)
}

/// Atomically archive a session by setting archived=true and file_exists=false.
///
/// Reads the existing session document to preserve metadata, deletes all docs
/// for the session_id, then re-adds just the session document with updated flags.
/// Message docs are removed since the JSONL file is no longer accessible.
pub fn archive_session(
    session_id: &str,
    writer: &Arc<Mutex<IndexWriter>>,
    reader: &IndexReader,
    schema: &IndexSchema,
) -> Option<SessionListItem> {
    let existing = find_session_doc(session_id, reader, schema)?;

    // Build new session doc preserving all metadata
    let mut doc = TantivyDocument::new();
    doc.add_text(schema.session_id, session_id);
    doc.add_text(schema.doc_type, "session");

    // Copy text fields
    for field in [
        schema.project_path,
        schema.project_raw,
        schema.summary,
        schema.first_prompt,
        schema.git_branch,
        schema.model,
        schema.status,
        schema.jsonl_path,
    ] {
        doc.add_text(field, existing.get_str(field).unwrap_or(""));
    }

    // Copy numeric fields
    for field in [
        schema.message_count,
        schema.input_tokens,
        schema.output_tokens,
        schema.total_tokens,
        schema.turn_depth,
    ] {
        doc.add_u64(field, existing.get_u64_val(field).unwrap_or(0));
    }

    // Copy date fields
    if let Some(dt) = existing.get_date_val(schema.created_at) {
        doc.add_date(schema.created_at, dt);
    }
    if let Some(dt) = existing.get_date_val(schema.modified_at) {
        doc.add_date(schema.modified_at, dt);
    }

    // Set archive flags
    doc.add_bool(schema.archived, true);
    doc.add_bool(schema.file_exists, false);
    doc.add_bool(
        schema.has_tool_use,
        existing.get_bool_val(schema.has_tool_use).unwrap_or(false),
    );

    // Build result before consuming doc
    let item = session_doc_to_list_item(&doc, schema);

    // Atomic: delete all docs for session, re-add archived session doc, commit
    let mut w = writer.lock().ok()?;
    w.delete_term(Term::from_field_text(schema.session_id, session_id));
    if let Err(e) = w.add_document(doc) {
        warn!("Failed to add archived session doc: {e}");
        return None;
    }
    if let Err(e) = w.commit() {
        warn!("Failed to commit archive: {e}");
        return None;
    }
    drop(w);

    debug!("Archived session {session_id}");
    Some(item)
}

/// Start the filesystem watcher for incremental indexing.
///
/// Watches `watch_dir` recursively for JSONL file changes with 2-second debounce.
/// Returns the watcher handle (must be kept alive) and a merge thread join handle.
///
/// The merge thread commits every 5 minutes using the same `Arc<Mutex<IndexWriter>>`
/// to trigger segment compaction via the configured merge policy.
pub fn start_index_watcher(
    watch_dir: PathBuf,
    writer: Arc<Mutex<IndexWriter>>,
    reader: IndexReader,
    schema: IndexSchema,
    paused: Arc<AtomicBool>,
    app_handle: Option<tauri::AppHandle>,
) -> Result<(RecommendedWatcher, std::thread::JoinHandle<()>), notify::Error> {
    let debounce_map: Arc<Mutex<HashMap<PathBuf, Instant>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let debounce_dur = Duration::from_secs(2);

    let watcher_writer = writer.clone();
    let watcher_schema = schema;
    let watcher_paused = paused.clone();

    let mut watcher =
        notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            // Check pause flag at top of callback
            if watcher_paused.load(Ordering::Relaxed) {
                return;
            }

            let event = match res {
                Ok(e) => e,
                Err(e) => {
                    warn!("Watcher error: {e}");
                    return;
                }
            };

            for path in &event.paths {
                if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }

                // 2s debounce: skip if last processed < 2s ago
                {
                    let mut map = debounce_map.lock().unwrap();
                    let now = Instant::now();
                    if let Some(last) = map.get(path) {
                        if now.duration_since(*last) < debounce_dur {
                            continue;
                        }
                    }
                    map.insert(path.clone(), now);
                }

                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        if let Some(item) =
                            reindex_session(path, &watcher_writer, &watcher_schema)
                        {
                            emit_session_updated(&app_handle, &item);
                        }
                    }
                    EventKind::Remove(_) => {
                        if let Some(session_id) =
                            path.file_stem().and_then(|s| s.to_str())
                        {
                            if let Some(item) = archive_session(
                                session_id,
                                &watcher_writer,
                                &reader,
                                &watcher_schema,
                            ) {
                                emit_session_updated(&app_handle, &item);
                            }
                        }
                    }
                    _ => {}
                }
            }
        })?;

    watcher.watch(&watch_dir, RecursiveMode::Recursive)?;

    // 5-minute merge thread using the SAME Arc<Mutex<IndexWriter>>
    let merge_writer = writer;
    let merge_paused = paused;
    let merge_handle = std::thread::Builder::new()
        .name("tantivy-merge".into())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_secs(300));
            if merge_paused.load(Ordering::Relaxed) {
                continue;
            }
            if let Ok(mut w) = merge_writer.lock() {
                if let Err(e) = w.commit() {
                    warn!("Merge commit failed: {e}");
                }
            }
        })
        .expect("Failed to spawn merge thread");

    Ok((watcher, merge_handle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::doc_ext::DocExt;
    use crate::search::schema::IndexSchema;
    use tantivy::Index;
    use tempfile::TempDir;

    fn create_test_index() -> (TempDir, Index, IndexSchema) {
        let schema = IndexSchema::new();
        let tmp = TempDir::new().unwrap();
        let index = Index::create_in_dir(tmp.path(), schema.schema.clone()).unwrap();
        (tmp, index, schema)
    }

    fn add_test_session(
        writer: &mut IndexWriter,
        schema: &IndexSchema,
        session_id: &str,
        archived: bool,
        file_exists: bool,
    ) {
        let mut doc = TantivyDocument::new();
        doc.add_text(schema.session_id, session_id);
        doc.add_text(schema.doc_type, "session");
        doc.add_text(schema.project_path, "/home/user/project");
        doc.add_text(schema.project_raw, "/home/user/project");
        doc.add_text(schema.summary, "Test session");
        doc.add_text(schema.first_prompt, "Hello");
        doc.add_text(schema.git_branch, "main");
        doc.add_text(schema.model, "claude-opus-4-6");
        doc.add_text(schema.status, "idle");
        doc.add_text(schema.jsonl_path, "/tmp/test.jsonl");
        doc.add_u64(schema.message_count, 5);
        doc.add_u64(schema.input_tokens, 100);
        doc.add_u64(schema.output_tokens, 200);
        doc.add_u64(schema.total_tokens, 300);
        doc.add_u64(schema.turn_depth, 3);
        doc.add_bool(schema.archived, archived);
        doc.add_bool(schema.file_exists, file_exists);
        doc.add_bool(schema.has_tool_use, true);
        writer.add_document(doc).unwrap();
    }

    fn add_test_message(
        writer: &mut IndexWriter,
        schema: &IndexSchema,
        session_id: &str,
    ) {
        let mut doc = TantivyDocument::new();
        doc.add_text(schema.session_id, session_id);
        doc.add_text(schema.doc_type, "message");
        doc.add_text(schema.role, "user");
        doc.add_text(schema.content, "Hello world");
        doc.add_text(schema.content_stored, "Hello world");
        doc.add_text(schema.content_type, "text");
        doc.add_u64(schema.turn_index, 0);
        doc.add_u64(schema.block_index, 0);
        writer.add_document(doc).unwrap();
    }

    #[test]
    fn test_archive_session_sets_flags() {
        let (_tmp, index, schema) = create_test_index();
        let mut writer = index.writer(50_000_000).unwrap();

        // Add a session doc (archived=false, file_exists=true) and a message doc
        add_test_session(&mut writer, &schema, "test-archive-1", false, true);
        add_test_message(&mut writer, &schema, "test-archive-1");
        writer.commit().unwrap();

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::Manual)
            .try_into()
            .unwrap();

        let writer_arc = Arc::new(Mutex::new(writer));

        // Archive the session
        let result = archive_session("test-archive-1", &writer_arc, &reader, &schema);
        assert!(result.is_some(), "archive_session should return Some");

        let item = result.unwrap();
        assert!(item.archived, "archived flag should be true");
        assert!(!item.file_exists, "file_exists flag should be false");
        assert_eq!(item.session_id, "test-archive-1");
        assert_eq!(item.summary, "Test session");
        assert_eq!(item.model, "claude-opus-4-6");
        assert!(item.has_tool_use, "has_tool_use should be preserved");
        assert_eq!(item.message_count, 5);
        assert_eq!(item.total_tokens, 300);

        // Verify via reader: only 1 doc remains (session only, message removed)
        reader.reload().unwrap();
        let searcher = reader.searcher();
        let query = tantivy::query::TermQuery::new(
            Term::from_field_text(schema.session_id, "test-archive-1"),
            tantivy::schema::IndexRecordOption::Basic,
        );
        let top_docs = searcher
            .search(&query, &tantivy::collector::TopDocs::with_limit(100))
            .unwrap();
        assert_eq!(
            top_docs.len(),
            1,
            "Should have exactly 1 doc (session only, message removed)"
        );

        // Verify the remaining doc has correct flags
        let (_, addr) = &top_docs[0];
        let stored_doc: TantivyDocument = searcher.doc(*addr).unwrap();
        assert_eq!(stored_doc.get_str(schema.doc_type), Some("session"));
        assert_eq!(stored_doc.get_bool_val(schema.archived), Some(true));
        assert_eq!(stored_doc.get_bool_val(schema.file_exists), Some(false));
    }

    #[test]
    fn test_archive_session_nonexistent_returns_none() {
        let (_tmp, index, schema) = create_test_index();
        let mut writer = index.writer(50_000_000).unwrap();
        writer.commit().unwrap();

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::Manual)
            .try_into()
            .unwrap();
        let writer_arc = Arc::new(Mutex::new(writer));

        let result = archive_session("nonexistent", &writer_arc, &reader, &schema);
        assert!(result.is_none(), "archive of nonexistent session should return None");
    }

    #[test]
    fn test_session_doc_to_list_item() {
        let schema = IndexSchema::new();
        let mut doc = TantivyDocument::new();
        doc.add_text(schema.session_id, "sid-1");
        doc.add_text(schema.doc_type, "session");
        doc.add_text(schema.project_path, "/project");
        doc.add_text(schema.summary, "My summary");
        doc.add_text(schema.first_prompt, "Hello");
        doc.add_text(schema.git_branch, "feature");
        doc.add_text(schema.model, "opus");
        doc.add_text(schema.status, "idle");
        doc.add_u64(schema.message_count, 10);
        doc.add_u64(schema.total_tokens, 500);
        doc.add_bool(schema.has_tool_use, false);
        doc.add_bool(schema.file_exists, true);
        doc.add_bool(schema.archived, false);

        let item = session_doc_to_list_item(&doc, &schema);
        assert_eq!(item.session_id, "sid-1");
        assert_eq!(item.project_path, "/project");
        assert_eq!(item.summary, "My summary");
        assert_eq!(item.message_count, 10);
        assert!(!item.archived);
        assert!(item.file_exists);
    }

    #[test]
    fn test_reindex_session_from_jsonl() {
        let (_tmp, index, schema) = create_test_index();
        let mut writer = index.writer(50_000_000).unwrap();

        // Add an old session doc
        add_test_session(&mut writer, &schema, "test-uuid-1", false, true);
        writer.commit().unwrap();

        let writer_arc = Arc::new(Mutex::new(writer));

        // Write a JSONL fixture file
        let jsonl_dir = TempDir::new().unwrap();
        let jsonl_content = concat!(
            r#"{"type":"user","message":{"role":"user","content":"Reindex me"},"timestamp":"2026-02-18T10:00:00Z","sessionId":"test-uuid-1","cwd":"/home/user/proj","gitBranch":"dev"}"#,
            "\n",
            r#"{"type":"assistant","message":{"role":"assistant","content":"Done!","model":"claude-opus-4-6","usage":{"input_tokens":50,"output_tokens":20}},"timestamp":"2026-02-18T10:01:00Z","sessionId":"test-uuid-1"}"#,
            "\n",
        );
        let jsonl_path = jsonl_dir.path().join("test-uuid-1.jsonl");
        std::fs::write(&jsonl_path, jsonl_content).unwrap();

        let result = reindex_session(&jsonl_path, &writer_arc, &schema);
        assert!(result.is_some());

        let item = result.unwrap();
        assert_eq!(item.session_id, "test-uuid-1");
        assert_eq!(item.git_branch, "dev");
        assert!(!item.archived);
        assert!(item.file_exists);
    }
}
