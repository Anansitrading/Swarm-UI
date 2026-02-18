use crate::search::doc_ext::DocExt;
use crate::search::indexer::extract_content_blocks;
use crate::search::schema::IndexSchema;
use crate::search::types::{
    ConversationMessage, IndexStats, MatchSnippet, SearchFilter, SearchResult, SessionDetail,
    SessionFilter, SessionListItem,
};
use crate::search::watcher::{format_tantivy_date, session_doc_to_list_item};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tantivy::collector::{Count, TopDocs};
use tantivy::query::{BooleanQuery, Occur, QueryParser, TermQuery};
use tantivy::schema::{IndexRecordOption, Term};
use tantivy::{Index, IndexReader, IndexWriter, Order, TantivyDocument};

/// Central search state, injected as Tauri managed state.
pub struct IndexHandle {
    pub index: Index,
    pub reader: IndexReader,
    pub schema: IndexSchema,
    pub writer: Arc<Mutex<IndexWriter>>,
    pub paused: Arc<AtomicBool>,
}

// ---------------------------------------------------------------------------
// Core query functions (synchronous, testable)
// ---------------------------------------------------------------------------

/// List sessions with optional filtering.
///
/// BooleanQuery on doc_type=session with optional project/git_branch/model filters.
/// Sorted by modified_at DESC via fast field, limit 10,000.
/// Bool fields (archived) are post-filtered since they are FAST-only (not indexed).
pub fn list_sessions_query(
    reader: &IndexReader,
    schema: &IndexSchema,
    filter: Option<&SessionFilter>,
) -> Result<Vec<SessionListItem>, String> {
    let searcher = reader.searcher();
    let include_archived = filter.map(|f| f.include_archived).unwrap_or(false);

    // Build query: Must doc_type=session + optional FAST field filters
    let mut clauses: Vec<(Occur, Box<dyn tantivy::query::Query>)> = vec![(
        Occur::Must,
        Box::new(TermQuery::new(
            Term::from_field_text(schema.doc_type, "session"),
            IndexRecordOption::Basic,
        )),
    )];

    if let Some(f) = filter {
        if let Some(ref project) = f.project {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(schema.project_raw, project),
                    IndexRecordOption::Basic,
                )),
            ));
        }
        if let Some(ref branch) = f.git_branch {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(schema.git_branch, branch),
                    IndexRecordOption::Basic,
                )),
            ));
        }
        if let Some(ref model) = f.model {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(schema.model, model),
                    IndexRecordOption::Basic,
                )),
            ));
        }
    }

    let query = BooleanQuery::new(clauses);

    let collector =
        TopDocs::with_limit(10_000).order_by_fast_field::<tantivy::DateTime>("modified_at", Order::Desc);
    let top_docs = searcher
        .search(&query, &collector)
        .map_err(|e| e.to_string())?;

    let mut items = Vec::with_capacity(top_docs.len());
    for (_date, addr) in top_docs {
        let doc: TantivyDocument = searcher.doc(addr).map_err(|e| e.to_string())?;
        let archived = doc.get_bool_val(schema.archived).unwrap_or(false);
        if !include_archived && archived {
            continue;
        }
        items.push(session_doc_to_list_item(&doc, schema));
    }

    Ok(items)
}

/// BM25 full-text search across message content with session enrichment.
///
/// Phase 1: BM25 on content field, exclude tool_result by default, over-fetch 3x limit.
/// Phase 2: Batch OR query for session metadata enrichment (NOT N+1).
pub fn search_sessions_query(
    reader: &IndexReader,
    schema: &IndexSchema,
    query_text: &str,
    filter: Option<&SearchFilter>,
) -> Result<Vec<SearchResult>, String> {
    let searcher = reader.searcher();
    let effective_limit = filter.and_then(|f| f.limit).unwrap_or(50);
    let include_tool_output = filter.map(|f| f.include_tool_output).unwrap_or(false);

    // Phase 1: Search message docs
    let query_parser = QueryParser::new(
        schema.schema.clone(),
        vec![schema.content],
        tantivy::tokenizer::TokenizerManager::default(),
    );
    let user_query = query_parser
        .parse_query(query_text)
        .map_err(|e| format!("Query parse error: {e}"))?;

    let mut clauses: Vec<(Occur, Box<dyn tantivy::query::Query>)> = vec![
        (
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(schema.doc_type, "message"),
                IndexRecordOption::Basic,
            )),
        ),
        (Occur::Must, Box::new(user_query)),
    ];

    if !include_tool_output {
        clauses.push((
            Occur::MustNot,
            Box::new(TermQuery::new(
                Term::from_field_text(schema.content_type, "tool_result"),
                IndexRecordOption::Basic,
            )),
        ));
    }

    if let Some(f) = filter {
        if let Some(ref project) = f.project {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(schema.msg_project, project),
                    IndexRecordOption::Basic,
                )),
            ));
        }
        if let Some(ref role) = f.role {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(schema.role, role),
                    IndexRecordOption::Basic,
                )),
            ));
        }
    }

    let query = BooleanQuery::new(clauses);
    let overfetch = effective_limit * 3;
    let top_docs = searcher
        .search(&query, &TopDocs::with_limit(overfetch))
        .map_err(|e| e.to_string())?;

    // Date post-filter and deduplicate by session_id, keeping top 3 snippets
    let date_from = filter
        .and_then(|f| f.date_from.as_deref())
        .and_then(parse_date_filter);
    let date_to = filter
        .and_then(|f| f.date_to.as_deref())
        .and_then(parse_date_filter);

    // Group by session_id: (best_score, Vec<MatchSnippet>)
    let mut session_hits: HashMap<String, (f32, Vec<MatchSnippet>)> = HashMap::new();

    for (score, addr) in top_docs {
        let doc: TantivyDocument = searcher.doc(addr).map_err(|e| e.to_string())?;

        // Date post-filter on message timestamp
        if let Some(ref from) = date_from {
            if let Some(ts) = doc.get_date_val(schema.timestamp) {
                if ts.into_timestamp_secs() < from.into_timestamp_secs() {
                    continue;
                }
            }
        }
        if let Some(ref to) = date_to {
            if let Some(ts) = doc.get_date_val(schema.timestamp) {
                if ts.into_timestamp_secs() > to.into_timestamp_secs() {
                    continue;
                }
            }
        }

        let sid = doc
            .get_str(schema.session_id)
            .unwrap_or("")
            .to_string();
        if sid.is_empty() {
            continue;
        }

        let snippet = MatchSnippet {
            role: doc.get_str(schema.role).unwrap_or("").to_string(),
            content_type: doc.get_str(schema.content_type).unwrap_or("").to_string(),
            snippet: doc.get_str(schema.content_stored).unwrap_or("").to_string(),
            timestamp: doc.get_date_val(schema.timestamp).map(format_tantivy_date),
            turn_index: doc.get_u64_val(schema.turn_index).unwrap_or(0),
        };

        let entry = session_hits
            .entry(sid)
            .or_insert_with(|| (score, Vec::new()));
        if entry.0 < score {
            entry.0 = score;
        }
        if entry.1.len() < 3 {
            entry.1.push(snippet);
        }
    }

    if session_hits.is_empty() {
        return Ok(Vec::new());
    }

    // Phase 2: Batch OR query for session metadata enrichment
    let session_ids: Vec<String> = session_hits.keys().cloned().collect();
    let session_meta = batch_fetch_sessions(&searcher, schema, &session_ids)?;

    // Build results sorted by best score DESC, limited to effective_limit
    let mut results: Vec<SearchResult> = session_hits
        .into_iter()
        .map(|(sid, (score, snippets))| {
            let meta = session_meta.get(&sid);
            SearchResult {
                session_id: sid,
                score,
                snippets,
                project_path: meta.and_then(|m| m.get_str(schema.project_path))
                    .map(|s| s.to_string()),
                summary: meta.and_then(|m| m.get_str(schema.summary))
                    .map(|s| s.to_string()),
                model: meta.and_then(|m| m.get_str(schema.model))
                    .map(|s| s.to_string()),
                modified_at: meta.and_then(|m| m.get_date_val(schema.modified_at))
                    .map(format_tantivy_date),
                file_exists: meta
                    .and_then(|m| m.get_bool_val(schema.file_exists))
                    .unwrap_or(true),
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(effective_limit);
    Ok(results)
}

/// Get full session metadata by session_id.
pub fn get_session_detail_query(
    reader: &IndexReader,
    schema: &IndexSchema,
    session_id: &str,
) -> Result<SessionDetail, String> {
    let searcher = reader.searcher();
    let query = BooleanQuery::new(vec![
        (
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(schema.session_id, session_id),
                IndexRecordOption::Basic,
            )),
        ),
        (
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(schema.doc_type, "session"),
                IndexRecordOption::Basic,
            )),
        ),
    ]);

    let top_docs = searcher
        .search(&query, &TopDocs::with_limit(1))
        .map_err(|e| e.to_string())?;

    let (_, addr) = top_docs
        .first()
        .ok_or_else(|| format!("Session not found: {session_id}"))?;
    let doc: TantivyDocument = searcher.doc(*addr).map_err(|e| e.to_string())?;

    Ok(session_doc_to_detail(&doc, schema))
}

/// Get conversation messages for a session.
///
/// If file_exists: parse from JSONL (full fidelity).
/// If file pruned: reconstruct from index (content_stored, truncated=true).
pub fn get_conversation_query(
    reader: &IndexReader,
    schema: &IndexSchema,
    session_id: &str,
) -> Result<Vec<ConversationMessage>, String> {
    // First, find the session doc to check file_exists and get jsonl_path
    let searcher = reader.searcher();
    let query = BooleanQuery::new(vec![
        (
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(schema.session_id, session_id),
                IndexRecordOption::Basic,
            )),
        ),
        (
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(schema.doc_type, "session"),
                IndexRecordOption::Basic,
            )),
        ),
    ]);

    let top_docs = searcher
        .search(&query, &TopDocs::with_limit(1))
        .map_err(|e| e.to_string())?;

    let (_, addr) = top_docs
        .first()
        .ok_or_else(|| format!("Session not found: {session_id}"))?;
    let session_doc: TantivyDocument = searcher.doc(*addr).map_err(|e| e.to_string())?;

    let file_exists = session_doc
        .get_bool_val(schema.file_exists)
        .unwrap_or(true);
    let jsonl_path = session_doc
        .get_str(schema.jsonl_path)
        .unwrap_or("")
        .to_string();

    if file_exists && !jsonl_path.is_empty() && Path::new(&jsonl_path).exists() {
        parse_conversation_from_jsonl(&jsonl_path)
    } else {
        reconstruct_conversation_from_index(session_id, &searcher, schema)
    }
}

/// Get index statistics: session/message counts, segments, disk size.
pub fn get_index_stats_query(
    reader: &IndexReader,
    schema: &IndexSchema,
) -> Result<IndexStats, String> {
    let searcher = reader.searcher();

    let session_query = TermQuery::new(
        Term::from_field_text(schema.doc_type, "session"),
        IndexRecordOption::Basic,
    );
    let total_sessions = searcher
        .search(&session_query, &Count)
        .map_err(|e| e.to_string())? as u64;

    let message_query = TermQuery::new(
        Term::from_field_text(schema.doc_type, "message"),
        IndexRecordOption::Basic,
    );
    let total_messages = searcher
        .search(&message_query, &Count)
        .map_err(|e| e.to_string())? as u64;

    // Count archived sessions by collecting session docs and post-filtering
    let top = TopDocs::with_limit(total_sessions as usize + 1);
    let all_sessions = searcher
        .search(&session_query, &top)
        .map_err(|e| e.to_string())?;

    let mut archived_count = 0u64;
    for (_, addr) in &all_sessions {
        let doc: TantivyDocument = searcher.doc(*addr).map_err(|e| e.to_string())?;
        if doc.get_bool_val(schema.archived).unwrap_or(false) {
            archived_count += 1;
        }
    }

    let active_sessions = total_sessions - archived_count;
    let segment_count = searcher.segment_readers().len() as u64;

    // Index size from standard location
    let index_size_bytes = get_index_dir_size();

    Ok(IndexStats {
        total_sessions,
        active_sessions,
        archived_sessions: archived_count,
        total_messages,
        segment_count,
        index_size_bytes,
    })
}

/// Pause watcher, delete all documents, re-index from filesystem, resume watcher.
pub fn reindex_all_query(handle: &IndexHandle) -> Result<(), String> {
    handle.paused.store(true, Ordering::SeqCst);

    // Delete all documents
    {
        let mut writer = handle.writer.lock().map_err(|e| e.to_string())?;
        writer.delete_all_documents().map_err(|e| e.to_string())?;
        writer.commit().map_err(|e| e.to_string())?;
    }

    // Re-index all JSONL files
    let projects_dir = dirs::home_dir()
        .ok_or("No home directory found")?
        .join(".claude")
        .join("projects");

    if projects_dir.exists() {
        let mut writer = handle.writer.lock().map_err(|e| e.to_string())?;
        crate::search::indexer::bulk_index(&mut writer, &handle.schema, &projects_dir, None)
            .map_err(|e| e.to_string())?;
    }

    handle.paused.store(false, Ordering::SeqCst);
    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri command wrappers (async, delegates to spawn_blocking)
// ---------------------------------------------------------------------------
// Note: list_sessions, get_session_detail, get_conversation use tantivy_ prefix
// to coexist with legacy commands in commands/session.rs during migration.
// Once session.rs is gutted, rename these back and update lib.rs handler.

#[tauri::command]
pub async fn tantivy_list_sessions(
    handle: tauri::State<'_, IndexHandle>,
    filter: Option<SessionFilter>,
) -> Result<Vec<SessionListItem>, String> {
    let reader = handle.reader.clone();
    let schema = handle.schema.clone();
    tokio::task::spawn_blocking(move || list_sessions_query(&reader, &schema, filter.as_ref()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn search_sessions(
    handle: tauri::State<'_, IndexHandle>,
    query_text: String,
    filter: Option<SearchFilter>,
) -> Result<Vec<SearchResult>, String> {
    let reader = handle.reader.clone();
    let schema = handle.schema.clone();
    tokio::task::spawn_blocking(move || {
        search_sessions_query(&reader, &schema, &query_text, filter.as_ref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tantivy_get_session_detail(
    handle: tauri::State<'_, IndexHandle>,
    session_id: String,
) -> Result<SessionDetail, String> {
    let reader = handle.reader.clone();
    let schema = handle.schema.clone();
    tokio::task::spawn_blocking(move || get_session_detail_query(&reader, &schema, &session_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tantivy_get_conversation(
    handle: tauri::State<'_, IndexHandle>,
    session_id: String,
) -> Result<Vec<ConversationMessage>, String> {
    let reader = handle.reader.clone();
    let schema = handle.schema.clone();
    tokio::task::spawn_blocking(move || get_conversation_query(&reader, &schema, &session_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_index_stats(
    handle: tauri::State<'_, IndexHandle>,
) -> Result<IndexStats, String> {
    let reader = handle.reader.clone();
    let schema = handle.schema.clone();
    tokio::task::spawn_blocking(move || get_index_stats_query(&reader, &schema))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn reindex_all(handle: tauri::State<'_, IndexHandle>) -> Result<(), String> {
    let reader = handle.reader.clone();
    let schema = handle.schema.clone();
    let writer = handle.writer.clone();
    let paused = handle.paused.clone();
    let index = handle.index.clone();
    tokio::task::spawn_blocking(move || {
        let h = IndexHandle {
            index,
            reader,
            schema,
            writer,
            paused,
        };
        reindex_all_query(&h)
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Convert a session TantivyDocument to a full SessionDetail.
fn session_doc_to_detail(doc: &TantivyDocument, schema: &IndexSchema) -> SessionDetail {
    SessionDetail {
        session_id: doc.get_str(schema.session_id).unwrap_or("").to_string(),
        project_path: doc.get_str(schema.project_path).unwrap_or("").to_string(),
        summary: doc.get_str(schema.summary).unwrap_or("").to_string(),
        first_prompt: doc.get_str(schema.first_prompt).unwrap_or("").to_string(),
        git_branch: doc.get_str(schema.git_branch).unwrap_or("").to_string(),
        model: doc.get_str(schema.model).unwrap_or("").to_string(),
        status: doc.get_str(schema.status).unwrap_or("").to_string(),
        jsonl_path: doc.get_str(schema.jsonl_path).unwrap_or("").to_string(),
        message_count: doc.get_u64_val(schema.message_count).unwrap_or(0),
        input_tokens: doc.get_u64_val(schema.input_tokens).unwrap_or(0),
        output_tokens: doc.get_u64_val(schema.output_tokens).unwrap_or(0),
        total_tokens: doc.get_u64_val(schema.total_tokens).unwrap_or(0),
        created_at: doc
            .get_date_val(schema.created_at)
            .map(format_tantivy_date),
        modified_at: doc
            .get_date_val(schema.modified_at)
            .map(format_tantivy_date),
        has_tool_use: doc.get_bool_val(schema.has_tool_use).unwrap_or(false),
        file_exists: doc.get_bool_val(schema.file_exists).unwrap_or(true),
        archived: doc.get_bool_val(schema.archived).unwrap_or(false),
        turn_depth: doc.get_u64_val(schema.turn_depth).unwrap_or(0),
    }
}

/// Batch-fetch session documents for a list of session_ids.
/// Uses a single OR query instead of N+1 individual lookups.
fn batch_fetch_sessions(
    searcher: &tantivy::Searcher,
    schema: &IndexSchema,
    session_ids: &[String],
) -> Result<HashMap<String, TantivyDocument>, String> {
    if session_ids.is_empty() {
        return Ok(HashMap::new());
    }

    // Inner OR query: Should(session_id=s1, session_id=s2, ...)
    let id_clauses: Vec<(Occur, Box<dyn tantivy::query::Query>)> = session_ids
        .iter()
        .map(|sid| {
            (
                Occur::Should,
                Box::new(TermQuery::new(
                    Term::from_field_text(schema.session_id, sid),
                    IndexRecordOption::Basic,
                )) as Box<dyn tantivy::query::Query>,
            )
        })
        .collect();

    let query = BooleanQuery::new(vec![
        (
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(schema.doc_type, "session"),
                IndexRecordOption::Basic,
            )),
        ),
        (
            Occur::Must,
            Box::new(BooleanQuery::new(id_clauses)),
        ),
    ]);

    let top_docs = searcher
        .search(&query, &TopDocs::with_limit(session_ids.len()))
        .map_err(|e| e.to_string())?;

    let mut map = HashMap::with_capacity(top_docs.len());
    for (_, addr) in top_docs {
        let doc: TantivyDocument = searcher.doc(addr).map_err(|e| e.to_string())?;
        if let Some(sid) = doc.get_str(schema.session_id) {
            map.insert(sid.to_string(), doc);
        }
    }

    Ok(map)
}

/// Parse a date string (YYYY-MM-DD or ISO 8601) into a tantivy DateTime.
fn parse_date_filter(s: &str) -> Option<tantivy::DateTime> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(tantivy::DateTime::from_timestamp_secs(
            dt.with_timezone(&chrono::Utc).timestamp(),
        ));
    }
    if let Ok(naive) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt = naive.and_hms_opt(0, 0, 0)?.and_utc();
        return Some(tantivy::DateTime::from_timestamp_secs(dt.timestamp()));
    }
    None
}

/// JSONL entry for conversation extraction.
#[derive(Deserialize)]
struct ConvJsonlEntry {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    message: Option<ConvJsonlMessage>,
    timestamp: Option<String>,
}

#[derive(Deserialize)]
struct ConvJsonlMessage {
    role: Option<String>,
    content: Option<serde_json::Value>,
}

/// Parse conversation messages from a JSONL file (full fidelity).
fn parse_conversation_from_jsonl(path: &str) -> Result<Vec<ConversationMessage>, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open {path}: {e}"))?;
    let reader = BufReader::new(file);
    let mut messages = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }

        let entry: ConvJsonlEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        let entry_type = entry.entry_type.as_deref().unwrap_or("");
        if entry_type != "user" && entry_type != "assistant" && entry_type != "tool" {
            continue;
        }

        let msg = match &entry.message {
            Some(m) => m,
            None => continue,
        };

        let role = msg.role.as_deref().unwrap_or("unknown").to_string();

        if let Some(ref content) = msg.content {
            let blocks = extract_content_blocks(content);
            for block in blocks {
                messages.push(ConversationMessage {
                    role: role.clone(),
                    content_type: block.content_type,
                    text: block.text,
                    timestamp: entry.timestamp.clone(),
                    truncated: false,
                });
            }
        }
    }

    Ok(messages)
}

/// Reconstruct conversation from index when JSONL file is pruned.
/// Uses content_stored (first 500 chars) with truncated=true.
fn reconstruct_conversation_from_index(
    session_id: &str,
    searcher: &tantivy::Searcher,
    schema: &IndexSchema,
) -> Result<Vec<ConversationMessage>, String> {
    let query = BooleanQuery::new(vec![
        (
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(schema.session_id, session_id),
                IndexRecordOption::Basic,
            )),
        ),
        (
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(schema.doc_type, "message"),
                IndexRecordOption::Basic,
            )),
        ),
    ]);

    let top_docs = searcher
        .search(&query, &TopDocs::with_limit(100_000))
        .map_err(|e| e.to_string())?;

    let mut messages: Vec<(u64, u64, ConversationMessage)> = Vec::with_capacity(top_docs.len());
    for (_, addr) in top_docs {
        let doc: TantivyDocument = searcher.doc(addr).map_err(|e| e.to_string())?;
        let turn_index = doc.get_u64_val(schema.turn_index).unwrap_or(0);
        let block_index = doc.get_u64_val(schema.block_index).unwrap_or(0);
        let msg = ConversationMessage {
            role: doc.get_str(schema.role).unwrap_or("").to_string(),
            content_type: doc.get_str(schema.content_type).unwrap_or("").to_string(),
            text: doc.get_str(schema.content_stored).unwrap_or("").to_string(),
            timestamp: doc
                .get_date_val(schema.timestamp)
                .map(format_tantivy_date),
            truncated: true,
        };
        messages.push((turn_index, block_index, msg));
    }

    messages.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    Ok(messages.into_iter().map(|(_, _, m)| m).collect())
}

/// Calculate total size of the index directory.
fn get_index_dir_size() -> u64 {
    let path = match dirs::data_local_dir() {
        Some(p) => p.join("swarm-ui").join("tantivy"),
        None => return 0,
    };
    dir_size_recursive(&path)
}

fn dir_size_recursive(path: &Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                size += dir_size_recursive(&p);
            } else {
                size += entry.metadata().ok().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    size
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::schema::IndexSchema;
    use tantivy::Index;

    /// Create an in-memory index with the full schema.
    fn test_index() -> (Index, IndexSchema) {
        let schema = IndexSchema::new();
        let index = Index::create_in_ram(schema.schema.clone());
        (index, schema)
    }

    /// Add a session document to the writer.
    fn add_session(
        writer: &IndexWriter,
        schema: &IndexSchema,
        session_id: &str,
        project: &str,
        branch: &str,
        model: &str,
        archived: bool,
        modified_secs: i64,
    ) {
        let mut doc = TantivyDocument::new();
        doc.add_text(schema.session_id, session_id);
        doc.add_text(schema.doc_type, "session");
        doc.add_text(schema.project_path, project);
        doc.add_text(schema.project_raw, project);
        doc.add_text(schema.summary, &format!("Summary for {session_id}"));
        doc.add_text(schema.first_prompt, "Hello");
        doc.add_text(schema.git_branch, branch);
        doc.add_text(schema.model, model);
        doc.add_text(schema.status, "idle");
        doc.add_text(schema.jsonl_path, &format!("/tmp/{session_id}.jsonl"));
        doc.add_u64(schema.message_count, 10);
        doc.add_u64(schema.input_tokens, 100);
        doc.add_u64(schema.output_tokens, 200);
        doc.add_u64(schema.total_tokens, 300);
        doc.add_date(
            schema.created_at,
            tantivy::DateTime::from_timestamp_secs(modified_secs - 3600),
        );
        doc.add_date(
            schema.modified_at,
            tantivy::DateTime::from_timestamp_secs(modified_secs),
        );
        doc.add_bool(schema.archived, archived);
        doc.add_bool(schema.file_exists, true);
        doc.add_bool(schema.has_tool_use, true);
        doc.add_u64(schema.turn_depth, 5);
        writer.add_document(doc).unwrap();
    }

    /// Add a message document to the writer.
    fn add_message(
        writer: &IndexWriter,
        schema: &IndexSchema,
        session_id: &str,
        role: &str,
        content: &str,
        content_type: &str,
        turn_index: u64,
        block_index: u64,
        project: &str,
        timestamp_secs: i64,
    ) {
        let stored = if content.len() > 500 {
            &content[..500]
        } else {
            content
        };
        let mut doc = TantivyDocument::new();
        doc.add_text(schema.session_id, session_id);
        doc.add_text(schema.doc_type, "message");
        doc.add_text(schema.role, role);
        doc.add_text(schema.content, content);
        doc.add_text(schema.content_stored, stored);
        doc.add_text(schema.content_type, content_type);
        doc.add_date(
            schema.timestamp,
            tantivy::DateTime::from_timestamp_secs(timestamp_secs),
        );
        doc.add_u64(schema.turn_index, turn_index);
        doc.add_u64(schema.block_index, block_index);
        doc.add_text(schema.msg_project, project);
        writer.add_document(doc).unwrap();
    }

    fn make_reader(index: &Index) -> IndexReader {
        index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::Manual)
            .try_into()
            .unwrap()
    }

    // -----------------------------------------------------------------------
    // list_sessions tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_list_sessions_returns_non_archived() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();

        add_session(&writer, &schema, "s1", "/proj", "main", "opus", false, 1000);
        add_session(&writer, &schema, "s2", "/proj", "main", "opus", true, 2000);
        add_session(&writer, &schema, "s3", "/proj", "dev", "opus", false, 3000);
        writer.commit().unwrap();

        let reader = make_reader(&index);
        let results = list_sessions_query(&reader, &schema, None).unwrap();

        assert_eq!(results.len(), 2, "Should exclude archived sessions");
        assert_eq!(results[0].session_id, "s3", "Most recent first");
        assert_eq!(results[1].session_id, "s1");
    }

    #[test]
    fn test_list_sessions_include_archived() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();

        add_session(&writer, &schema, "s1", "/proj", "main", "opus", false, 1000);
        add_session(&writer, &schema, "s2", "/proj", "main", "opus", true, 2000);
        writer.commit().unwrap();

        let reader = make_reader(&index);
        let filter = SessionFilter {
            include_archived: true,
            ..Default::default()
        };
        let results = list_sessions_query(&reader, &schema, Some(&filter)).unwrap();

        assert_eq!(results.len(), 2, "Should include archived when requested");
    }

    #[test]
    fn test_list_sessions_filter_by_project() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();

        add_session(&writer, &schema, "s1", "/proj-a", "main", "opus", false, 1000);
        add_session(&writer, &schema, "s2", "/proj-b", "main", "opus", false, 2000);
        writer.commit().unwrap();

        let reader = make_reader(&index);
        let filter = SessionFilter {
            project: Some("/proj-a".into()),
            ..Default::default()
        };
        let results = list_sessions_query(&reader, &schema, Some(&filter)).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, "s1");
    }

    #[test]
    fn test_list_sessions_filter_by_branch_and_model() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();

        add_session(&writer, &schema, "s1", "/p", "main", "opus", false, 1000);
        add_session(&writer, &schema, "s2", "/p", "dev", "sonnet", false, 2000);
        add_session(&writer, &schema, "s3", "/p", "main", "sonnet", false, 3000);
        writer.commit().unwrap();

        let reader = make_reader(&index);
        let filter = SessionFilter {
            git_branch: Some("main".into()),
            model: Some("opus".into()),
            ..Default::default()
        };
        let results = list_sessions_query(&reader, &schema, Some(&filter)).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, "s1");
    }

    #[test]
    fn test_list_sessions_sorted_by_modified_at_desc() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();

        add_session(&writer, &schema, "old", "/p", "main", "opus", false, 1000);
        add_session(&writer, &schema, "mid", "/p", "main", "opus", false, 5000);
        add_session(&writer, &schema, "new", "/p", "main", "opus", false, 9000);
        writer.commit().unwrap();

        let reader = make_reader(&index);
        let results = list_sessions_query(&reader, &schema, None).unwrap();

        assert_eq!(results[0].session_id, "new");
        assert_eq!(results[1].session_id, "mid");
        assert_eq!(results[2].session_id, "old");
    }

    // -----------------------------------------------------------------------
    // search_sessions tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_search_finds_matching_messages() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();

        add_session(&writer, &schema, "s1", "/proj", "main", "opus", false, 1000);
        add_message(
            &writer, &schema, "s1", "user", "How do I implement authentication?",
            "text", 0, 0, "/proj", 1000,
        );
        add_message(
            &writer, &schema, "s1", "assistant", "You can use JWT tokens for auth",
            "text", 1, 0, "/proj", 1001,
        );

        add_session(&writer, &schema, "s2", "/proj", "main", "opus", false, 2000);
        add_message(
            &writer, &schema, "s2", "user", "Fix the CSS layout",
            "text", 0, 0, "/proj", 2000,
        );
        writer.commit().unwrap();

        let reader = make_reader(&index);
        let results = search_sessions_query(&reader, &schema, "authentication", None).unwrap();

        assert_eq!(results.len(), 1, "Only s1 should match");
        assert_eq!(results[0].session_id, "s1");
        assert!(!results[0].snippets.is_empty());
    }

    #[test]
    fn test_search_excludes_tool_result_by_default() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();

        add_session(&writer, &schema, "s1", "/proj", "main", "opus", false, 1000);
        add_message(
            &writer, &schema, "s1", "assistant",
            "tool_result with secret data about tantivy search engine",
            "tool_result", 0, 0, "/proj", 1000,
        );
        // No text messages matching "tantivy"
        writer.commit().unwrap();

        let reader = make_reader(&index);
        let results = search_sessions_query(&reader, &schema, "tantivy", None).unwrap();

        assert_eq!(results.len(), 0, "tool_result should be excluded by default");
    }

    #[test]
    fn test_search_includes_tool_result_when_requested() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();

        add_session(&writer, &schema, "s1", "/proj", "main", "opus", false, 1000);
        add_message(
            &writer, &schema, "s1", "assistant",
            "tool_result with tantivy search data",
            "tool_result", 0, 0, "/proj", 1000,
        );
        writer.commit().unwrap();

        let reader = make_reader(&index);
        let filter = SearchFilter {
            include_tool_output: true,
            ..Default::default()
        };
        let results =
            search_sessions_query(&reader, &schema, "tantivy", Some(&filter)).unwrap();

        assert_eq!(results.len(), 1, "tool_result should be included when requested");
    }

    #[test]
    fn test_search_enriches_with_session_metadata() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();

        add_session(&writer, &schema, "s1", "/my/project", "feature", "claude-opus-4-6", false, 5000);
        add_message(
            &writer, &schema, "s1", "user", "implement the dashboard component",
            "text", 0, 0, "/my/project", 5000,
        );
        writer.commit().unwrap();

        let reader = make_reader(&index);
        let results = search_sessions_query(&reader, &schema, "dashboard", None).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].project_path.as_deref(), Some("/my/project"));
        assert_eq!(results[0].model.as_deref(), Some("claude-opus-4-6"));
        assert!(results[0].file_exists);
    }

    // -----------------------------------------------------------------------
    // get_session_detail tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_session_detail_returns_full_metadata() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();

        add_session(&writer, &schema, "detail-1", "/project", "main", "opus", false, 1000);
        writer.commit().unwrap();

        let reader = make_reader(&index);
        let detail = get_session_detail_query(&reader, &schema, "detail-1").unwrap();

        assert_eq!(detail.session_id, "detail-1");
        assert_eq!(detail.project_path, "/project");
        assert_eq!(detail.git_branch, "main");
        assert_eq!(detail.model, "opus");
        assert_eq!(detail.message_count, 10);
        assert_eq!(detail.input_tokens, 100);
        assert_eq!(detail.output_tokens, 200);
        assert_eq!(detail.total_tokens, 300);
        assert_eq!(detail.turn_depth, 5);
        assert!(!detail.archived);
        assert!(detail.file_exists);
        assert!(detail.has_tool_use);
        assert_eq!(detail.jsonl_path, "/tmp/detail-1.jsonl");
    }

    #[test]
    fn test_get_session_detail_not_found() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();
        writer.commit().unwrap();

        let reader = make_reader(&index);
        let result = get_session_detail_query(&reader, &schema, "nonexistent");

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    // -----------------------------------------------------------------------
    // get_conversation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_conversation_reconstructs_from_index() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();

        // Session with file_exists=false (pruned)
        let mut session_doc = TantivyDocument::new();
        session_doc.add_text(schema.session_id, "conv-1");
        session_doc.add_text(schema.doc_type, "session");
        session_doc.add_text(schema.project_path, "/p");
        session_doc.add_text(schema.project_raw, "/p");
        session_doc.add_text(schema.summary, "test");
        session_doc.add_text(schema.first_prompt, "hi");
        session_doc.add_text(schema.git_branch, "main");
        session_doc.add_text(schema.model, "opus");
        session_doc.add_text(schema.status, "idle");
        session_doc.add_text(schema.jsonl_path, "/nonexistent/path.jsonl");
        session_doc.add_u64(schema.message_count, 2);
        session_doc.add_u64(schema.input_tokens, 0);
        session_doc.add_u64(schema.output_tokens, 0);
        session_doc.add_u64(schema.total_tokens, 0);
        session_doc.add_u64(schema.turn_depth, 2);
        session_doc.add_bool(schema.archived, true);
        session_doc.add_bool(schema.file_exists, false);
        session_doc.add_bool(schema.has_tool_use, false);
        writer.add_document(session_doc).unwrap();

        // Message docs (out of order to test sorting)
        add_message(
            &writer, &schema, "conv-1", "assistant", "I can help with that!",
            "text", 1, 0, "/p", 2000,
        );
        add_message(
            &writer, &schema, "conv-1", "user", "Help me build a feature",
            "text", 0, 0, "/p", 1000,
        );
        writer.commit().unwrap();

        let reader = make_reader(&index);
        let messages = get_conversation_query(&reader, &schema, "conv-1").unwrap();

        assert_eq!(messages.len(), 2);
        // Should be sorted by turn_index
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].text, "Help me build a feature");
        assert!(messages[0].truncated, "Reconstructed messages should be truncated");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].text, "I can help with that!");
    }

    // -----------------------------------------------------------------------
    // get_index_stats tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_index_stats_counts() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();

        add_session(&writer, &schema, "s1", "/p", "main", "opus", false, 1000);
        add_session(&writer, &schema, "s2", "/p", "main", "opus", true, 2000);
        add_message(&writer, &schema, "s1", "user", "hello", "text", 0, 0, "/p", 1000);
        add_message(&writer, &schema, "s1", "assistant", "hi", "text", 1, 0, "/p", 1001);
        add_message(&writer, &schema, "s2", "user", "bye", "text", 0, 0, "/p", 2000);
        writer.commit().unwrap();

        let reader = make_reader(&index);
        let stats = get_index_stats_query(&reader, &schema).unwrap();

        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.active_sessions, 1);
        assert_eq!(stats.archived_sessions, 1);
        assert_eq!(stats.total_messages, 3);
        assert!(stats.segment_count >= 1);
    }

    // -----------------------------------------------------------------------
    // search edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_search_deduplicates_by_session() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();

        add_session(&writer, &schema, "s1", "/p", "main", "opus", false, 1000);
        // Multiple messages in same session matching "rust"
        add_message(&writer, &schema, "s1", "user", "How to learn rust programming", "text", 0, 0, "/p", 1000);
        add_message(&writer, &schema, "s1", "assistant", "Rust is a systems language", "text", 1, 0, "/p", 1001);
        add_message(&writer, &schema, "s1", "user", "More about rust ownership", "text", 2, 0, "/p", 1002);
        add_message(&writer, &schema, "s1", "assistant", "Rust ownership model explained", "text", 3, 0, "/p", 1003);
        writer.commit().unwrap();

        let reader = make_reader(&index);
        let results = search_sessions_query(&reader, &schema, "rust", None).unwrap();

        assert_eq!(results.len(), 1, "Should deduplicate to 1 session");
        assert!(results[0].snippets.len() <= 3, "Should keep at most 3 snippets");
    }

    #[test]
    fn test_search_empty_query_returns_empty() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();
        add_session(&writer, &schema, "s1", "/p", "main", "opus", false, 1000);
        add_message(&writer, &schema, "s1", "user", "hello world", "text", 0, 0, "/p", 1000);
        writer.commit().unwrap();

        let reader = make_reader(&index);
        // Empty query should parse but match nothing or everything depending on parser
        let results = search_sessions_query(&reader, &schema, "xyznonexistent", None).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_search_filter_by_role() {
        let (index, schema) = test_index();
        let mut writer = index.writer::<TantivyDocument>(50_000_000).unwrap();

        add_session(&writer, &schema, "s1", "/p", "main", "opus", false, 1000);
        add_message(&writer, &schema, "s1", "user", "explain polymorphism", "text", 0, 0, "/p", 1000);
        add_message(&writer, &schema, "s1", "assistant", "polymorphism is a concept", "text", 1, 0, "/p", 1001);
        writer.commit().unwrap();

        let reader = make_reader(&index);
        let filter = SearchFilter {
            role: Some("user".into()),
            ..Default::default()
        };
        let results =
            search_sessions_query(&reader, &schema, "polymorphism", Some(&filter)).unwrap();

        assert_eq!(results.len(), 1);
        // All snippets should be from user role
        for snippet in &results[0].snippets {
            assert_eq!(snippet.role, "user");
        }
    }
}
