use crate::search::schema::{IndexSchema, SCHEMA_VERSION};
use crate::search::types::IndexMeta;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use tantivy::TantivyDocument;

/// Metadata entry from sessions-index.json.
#[derive(Debug, Clone, Deserialize)]
pub struct SessionIndexEntry {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "fullPath")]
    pub full_path: Option<String>,
    #[serde(rename = "firstPrompt")]
    pub first_prompt: Option<String>,
    pub summary: Option<String>,
    #[serde(rename = "messageCount")]
    pub message_count: Option<u64>,
    pub created: Option<String>,
    pub modified: Option<String>,
    #[serde(rename = "gitBranch")]
    pub git_branch: Option<String>,
    #[serde(rename = "projectPath")]
    pub project_path: Option<String>,
}

/// sessions-index.json file structure.
#[derive(Debug, Deserialize)]
struct SessionsIndexFile {
    entries: Vec<SessionIndexEntry>,
}

/// A single extracted content block from a JSONL message.
#[derive(Debug, Clone)]
pub struct ContentBlock {
    pub content_type: String,
    pub text: String,
}

/// Extract content blocks (text, tool_use, tool_result, thinking) from a message content value.
pub fn extract_content_blocks(content: &serde_json::Value) -> Vec<ContentBlock> {
    let mut blocks = Vec::new();

    if let Some(s) = content.as_str() {
        if !s.is_empty() {
            blocks.push(ContentBlock {
                content_type: "text".to_string(),
                text: s.to_string(),
            });
        }
        return blocks;
    }

    if let Some(arr) = content.as_array() {
        for block in arr {
            let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match block_type {
                "text" => {
                    let text = block.get("text").and_then(|t| t.as_str()).unwrap_or("");
                    if !text.is_empty() {
                        blocks.push(ContentBlock {
                            content_type: "text".to_string(),
                            text: text.to_string(),
                        });
                    }
                }
                "tool_use" => {
                    let name = block
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("unknown");
                    let input = block
                        .get("input")
                        .map(|i| serde_json::to_string(i).unwrap_or_default())
                        .unwrap_or_default();
                    let text = format!("tool_use: {} {}", name, input);
                    blocks.push(ContentBlock {
                        content_type: "tool_use".to_string(),
                        text,
                    });
                }
                "tool_result" => {
                    let content_val = block.get("content");
                    let text = if let Some(arr2) = content_val.and_then(|c| c.as_array()) {
                        arr2.iter()
                            .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                            .collect::<Vec<_>>()
                            .join("\n")
                    } else if let Some(s) = content_val.and_then(|c| c.as_str()) {
                        s.to_string()
                    } else {
                        String::new()
                    };
                    if !text.is_empty() {
                        blocks.push(ContentBlock {
                            content_type: "tool_result".to_string(),
                            text,
                        });
                    }
                }
                "thinking" => {
                    let text = block
                        .get("thinking")
                        .or_else(|| block.get("text"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("");
                    if !text.is_empty() {
                        blocks.push(ContentBlock {
                            content_type: "thinking".to_string(),
                            text: text.to_string(),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    blocks
}

/// JSONL entry — minimal fields needed for indexing.
#[derive(Debug, Deserialize)]
struct IndexJsonlEntry {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    message: Option<IndexJsonlMessage>,
    timestamp: Option<String>,
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    #[serde(rename = "gitBranch")]
    git_branch: Option<String>,
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IndexJsonlMessage {
    role: Option<String>,
    content: Option<serde_json::Value>,
    usage: Option<IndexJsonlUsage>,
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IndexJsonlUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

/// Parse a single JSONL file into a Vec of TantivyDocuments.
/// The first document is always the session document; the rest are message documents.
///
/// `meta` is optional pre-populated metadata from sessions-index.json.
pub fn parse_jsonl_to_documents(
    path: &Path,
    schema: &IndexSchema,
    meta: Option<&SessionIndexEntry>,
) -> Vec<TantivyDocument> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let reader = BufReader::with_capacity(64 * 1024, file);

    let mut session_id = String::new();
    let mut cwd = String::new();
    let mut git_branch = String::new();
    let mut model = String::new();
    let mut input_tokens: u64 = 0;
    let mut output_tokens: u64 = 0;
    let mut has_tool_use = false;
    let mut first_prompt = String::new();
    let mut summary = String::new();
    let mut first_timestamp: Option<String> = None;
    let mut last_timestamp: Option<String> = None;
    let mut turn_index: u64 = 0;
    let mut message_count: u64 = 0;
    let mut status = "idle".to_string();

    // Pre-populate from sessions-index.json metadata if available
    if let Some(m) = meta {
        if let Some(ref fp) = m.first_prompt {
            first_prompt = fp.clone();
        }
        if let Some(ref s) = m.summary {
            summary = s.clone();
        }
        if let Some(ref gb) = m.git_branch {
            git_branch = gb.clone();
        }
        if let Some(ref pp) = m.project_path {
            cwd = pp.clone();
        }
    }

    let mut message_docs: Vec<TantivyDocument> = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }

        let entry: IndexJsonlEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Extract session-level metadata (first occurrence wins for most fields)
        if let Some(ref sid) = entry.session_id {
            if session_id.is_empty() {
                session_id = sid.clone();
            }
        }
        if let Some(ref c) = entry.cwd {
            if cwd.is_empty() && !c.is_empty() {
                cwd = c.clone();
            }
        }
        if let Some(ref b) = entry.git_branch {
            if git_branch.is_empty() && !b.is_empty() {
                git_branch = b.clone();
            }
        }
        if let Some(ref ts) = entry.timestamp {
            if first_timestamp.is_none() {
                first_timestamp = Some(ts.clone());
            }
            last_timestamp = Some(ts.clone());
        }

        let entry_type = entry.entry_type.as_deref().unwrap_or("");
        if entry_type != "user" && entry_type != "assistant" && entry_type != "tool" {
            continue;
        }

        let msg = match &entry.message {
            Some(m) => m,
            None => continue,
        };

        // Update model (always latest)
        if let Some(ref m) = msg.model {
            model = m.clone();
        }

        // Token aggregation: input_tokens = latest, output_tokens = cumulative
        if let Some(ref usage) = msg.usage {
            if let Some(it) = usage.input_tokens {
                input_tokens = it;
            }
            if let Some(ot) = usage.output_tokens {
                output_tokens += ot;
            }
        }

        let role = msg.role.as_deref().unwrap_or("unknown");

        // Status tracking
        match role {
            "user" => {
                status = "thinking".to_string();
            }
            "assistant" => {
                status = "idle".to_string();
            }
            _ => {}
        }

        // Extract content blocks and create message documents
        if let Some(ref content) = msg.content {
            let blocks = extract_content_blocks(content);
            let mut block_index: u64 = 0;

            // Extract first user prompt if not yet set
            if first_prompt.is_empty() && role == "user" {
                for b in &blocks {
                    if b.content_type == "text" && !b.text.is_empty() {
                        first_prompt = if b.text.len() > 500 {
                            truncate_at_char_boundary(&b.text, 500)
                        } else {
                            b.text.clone()
                        };
                        break;
                    }
                }
            }

            for block in &blocks {
                if block.content_type == "tool_use" || block.content_type == "tool_result" {
                    has_tool_use = true;
                }

                let content_stored = if block.text.len() > 500 {
                    truncate_at_char_boundary(&block.text, 500)
                } else {
                    block.text.clone()
                };

                let timestamp_str = entry.timestamp.as_deref().unwrap_or("");

                let mut doc = TantivyDocument::new();
                doc.add_text(schema.session_id, &session_id);
                doc.add_text(schema.doc_type, "message");
                doc.add_text(schema.role, role);
                doc.add_text(schema.content, &block.text);
                doc.add_text(schema.content_stored, &content_stored);
                doc.add_text(schema.content_type, &block.content_type);
                if let Some(dt) = parse_timestamp(timestamp_str) {
                    doc.add_date(schema.timestamp, dt);
                }
                doc.add_u64(schema.turn_index, turn_index);
                doc.add_u64(schema.block_index, block_index);
                doc.add_text(schema.msg_project, &cwd);

                message_docs.push(doc);
                block_index += 1;
                message_count += 1;
            }
        }

        turn_index += 1;
    }

    // If no summary from index metadata, use first_prompt as fallback
    if summary.is_empty() {
        summary = first_prompt.clone();
    }

    let total_tokens = input_tokens + output_tokens;
    let jsonl_path = path.to_string_lossy().to_string();

    // Build session document
    let mut session_doc = TantivyDocument::new();
    session_doc.add_text(schema.session_id, &session_id);
    session_doc.add_text(schema.doc_type, "session");
    session_doc.add_text(schema.project_path, &cwd);
    session_doc.add_text(schema.project_raw, &cwd);
    session_doc.add_text(schema.summary, &summary);
    session_doc.add_text(schema.first_prompt, &first_prompt);
    session_doc.add_text(schema.git_branch, &git_branch);
    session_doc.add_text(schema.model, &model);
    session_doc.add_text(schema.status, &status);
    session_doc.add_text(schema.jsonl_path, &jsonl_path);
    session_doc.add_u64(schema.message_count, message_count);
    session_doc.add_u64(schema.input_tokens, input_tokens);
    session_doc.add_u64(schema.output_tokens, output_tokens);
    session_doc.add_u64(schema.total_tokens, total_tokens);

    // Parse timestamps
    if let Some(ref ts) = first_timestamp {
        if let Some(dt) = parse_timestamp(ts) {
            session_doc.add_date(schema.created_at, dt);
        }
    } else if let Some(ref m) = meta {
        if let Some(ref c) = m.created {
            if let Some(dt) = parse_timestamp(c) {
                session_doc.add_date(schema.created_at, dt);
            }
        }
    }
    if let Some(ref ts) = last_timestamp {
        if let Some(dt) = parse_timestamp(ts) {
            session_doc.add_date(schema.modified_at, dt);
        }
    } else if let Some(ref m) = meta {
        if let Some(ref md) = m.modified {
            if let Some(dt) = parse_timestamp(md) {
                session_doc.add_date(schema.modified_at, dt);
            }
        }
    }

    session_doc.add_bool(schema.archived, false);
    session_doc.add_bool(schema.file_exists, true);
    session_doc.add_bool(schema.has_tool_use, has_tool_use);
    session_doc.add_u64(schema.turn_depth, turn_index);

    // Session doc first, then message docs
    let mut docs = Vec::with_capacity(1 + message_docs.len());
    docs.push(session_doc);
    docs.append(&mut message_docs);
    docs
}

/// Load all sessions-index.json files under a projects directory.
/// Returns a HashMap keyed by session_id.
pub fn load_all_index_files(projects_dir: &Path) -> HashMap<String, SessionIndexEntry> {
    let mut map = HashMap::new();

    let entries = match fs::read_dir(projects_dir) {
        Ok(e) => e,
        Err(_) => return map,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let index_path = path.join("sessions-index.json");
        if !index_path.exists() {
            continue;
        }
        let content = match fs::read_to_string(&index_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let index_file: SessionsIndexFile = match serde_json::from_str(&content) {
            Ok(f) => f,
            Err(_) => continue,
        };
        for entry in index_file.entries {
            map.insert(entry.session_id.clone(), entry);
        }
    }

    map
}

/// Recursively discover all .jsonl files under a projects directory.
pub fn discover_jsonl_files(projects_dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    discover_jsonl_recursive(projects_dir, &mut files);
    files
}

fn discover_jsonl_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            discover_jsonl_recursive(&path, files);
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }
}

/// Check whether the on-disk index has a schema version mismatch.
/// Returns `true` if a reindex is needed (missing file or version mismatch).
pub fn schema_version_mismatch(index_path: &Path) -> bool {
    let meta_path = index_path.join("swarm-ui-meta.json");
    let content = match fs::read_to_string(&meta_path) {
        Ok(c) => c,
        Err(_) => return true,
    };
    let meta: IndexMeta = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(_) => return true,
    };
    meta.schema_version != SCHEMA_VERSION
}

/// Write index metadata to swarm-ui-meta.json.
pub fn write_index_meta(index_path: &Path, session_count: u64) -> std::io::Result<()> {
    let meta = IndexMeta {
        schema_version: SCHEMA_VERSION,
        indexed_at: chrono::Utc::now().to_rfc3339(),
        session_count,
    };
    let json = serde_json::to_string_pretty(&meta)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(index_path.join("swarm-ui-meta.json"), json)
}

/// Bulk index all JSONL files using rayon for parallel parsing
/// and crossbeam_channel for feeding documents to the writer.
///
/// `app_handle` is optional — when provided, emits `index:progress` events.
pub fn bulk_index(
    writer: &mut tantivy::IndexWriter,
    schema: &IndexSchema,
    projects_dir: &Path,
    app_handle: Option<&tauri::AppHandle>,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    use crossbeam_channel::bounded;
    use rayon::prelude::*;

    // Phase 1: Discover JSONL files
    emit_progress(app_handle, "discovering", 0, 0);
    let jsonl_files = discover_jsonl_files(projects_dir);
    let total = jsonl_files.len() as u64;

    if total == 0 {
        return Ok(0);
    }

    // Phase 2: Load sessions-index.json metadata
    emit_progress(app_handle, "loading_metadata", 0, total);
    let index_meta = load_all_index_files(projects_dir);

    // Phase 3: Parallel parse + channel -> writer
    let (sender, receiver) = bounded::<Vec<TantivyDocument>>(64);
    let schema_clone = schema.clone();

    let producer = std::thread::spawn(move || {
        jsonl_files.par_iter().for_each(|path| {
            let file_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let meta = index_meta.get(file_stem);
            let docs = parse_jsonl_to_documents(path, &schema_clone, meta);
            if !docs.is_empty() {
                let _ = sender.send(docs);
            }
        });
        drop(sender);
    });

    // Consumer: write docs to index
    let mut session_count: u64 = 0;
    let mut processed: u64 = 0;
    for docs in receiver {
        for doc in docs {
            writer.add_document(doc)?;
        }
        session_count += 1;
        processed += 1;
        if processed % 500 == 0 {
            emit_progress(app_handle, "indexing", processed, total);
        }
    }

    producer.join().map_err(|_| "Producer thread panicked")?;

    // Phase 4: Commit
    emit_progress(app_handle, "committing", total, total);
    writer.commit()?;

    Ok(session_count)
}

fn emit_progress(app_handle: Option<&tauri::AppHandle>, phase: &str, current: u64, total: u64) {
    if let Some(handle) = app_handle {
        use tauri::Emitter;
        let payload = crate::search::types::IndexProgress {
            phase: phase.to_string(),
            current,
            total,
        };
        let _ = handle.emit("index:progress", &payload);
    }
}

/// Parse an ISO 8601 timestamp string to a Tantivy DateTime.
fn parse_timestamp(ts: &str) -> Option<tantivy::DateTime> {
    if ts.is_empty() {
        return None;
    }
    // Parse ISO 8601 with chrono, then convert to Tantivy DateTime
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
        let utc = dt.with_timezone(&chrono::Utc);
        return Some(tantivy::DateTime::from_timestamp_secs(utc.timestamp()));
    }
    // Try without timezone (assume UTC)
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S%.fZ") {
        let utc = naive.and_utc();
        return Some(tantivy::DateTime::from_timestamp_secs(utc.timestamp()));
    }
    // Try unix timestamp (milliseconds or seconds)
    if let Ok(num) = ts.parse::<u64>() {
        let secs = if num > 1_000_000_000_000 {
            (num / 1000) as i64
        } else {
            num as i64
        };
        return Some(tantivy::DateTime::from_timestamp_secs(secs));
    }
    None
}

/// Truncate a string at a char boundary, at or before `max_bytes`.
fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let end = s
        .char_indices()
        .take_while(|(i, _)| *i <= max_bytes)
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(max_bytes.min(s.len()));
    s[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::doc_ext::DocExt;
    use std::io::Write;
    use tempfile::TempDir;

    fn fixture_jsonl() -> &'static str {
        concat!(
            r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"Hello world"}]},"timestamp":"2026-02-18T10:00:00Z","sessionId":"test-uuid-1","cwd":"/home/devuser/project","gitBranch":"main"}"#,
            "\n",
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Hi there! Let me help."},{"type":"tool_use","id":"t1","name":"Read","input":{"path":"/tmp/test"}}],"model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":50}},"timestamp":"2026-02-18T10:01:00Z","sessionId":"test-uuid-1"}"#,
            "\n",
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_result","tool_use_id":"t1","content":"file contents here"}],"usage":{"input_tokens":200,"output_tokens":75}},"timestamp":"2026-02-18T10:01:30Z","sessionId":"test-uuid-1"}"#,
            "\n",
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"thinking","thinking":"Let me analyze this..."}],"usage":{"input_tokens":250,"output_tokens":30}},"timestamp":"2026-02-18T10:02:00Z","sessionId":"test-uuid-1"}"#,
            "\n",
        )
    }

    fn write_fixture(dir: &Path, filename: &str, content: &str) -> PathBuf {
        let path = dir.join(filename);
        let mut file = File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_parse_produces_session_and_message_docs() {
        let tmp = TempDir::new().unwrap();
        let path = write_fixture(tmp.path(), "test-uuid-1.jsonl", fixture_jsonl());
        let schema = IndexSchema::new();

        let docs = parse_jsonl_to_documents(&path, &schema, None);

        // 1 session doc + message docs (1 text user + 2 assistant text+tool_use + 1 tool_result + 1 thinking)
        assert!(!docs.is_empty(), "Should produce at least 1 doc");

        // First doc should be session
        let session_doc = &docs[0];
        assert_eq!(session_doc.get_str(schema.doc_type), Some("session"));
        assert_eq!(session_doc.get_str(schema.session_id), Some("test-uuid-1"));

        // Remaining docs should be messages
        let msg_count = docs.iter().skip(1).count();
        assert!(msg_count >= 4, "Expected at least 4 message docs, got {}", msg_count);

        for doc in docs.iter().skip(1) {
            assert_eq!(doc.get_str(schema.doc_type), Some("message"));
            assert_eq!(doc.get_str(schema.session_id), Some("test-uuid-1"));
        }
    }

    #[test]
    fn test_parse_extracts_content_types() {
        let tmp = TempDir::new().unwrap();
        let path = write_fixture(tmp.path(), "test-uuid-1.jsonl", fixture_jsonl());
        let schema = IndexSchema::new();

        let docs = parse_jsonl_to_documents(&path, &schema, None);

        let content_types: Vec<&str> = docs
            .iter()
            .skip(1) // skip session doc
            .filter_map(|d| d.get_str(schema.content_type))
            .collect();

        assert!(content_types.contains(&"text"), "Should have text blocks");
        assert!(content_types.contains(&"tool_use"), "Should have tool_use blocks");
        assert!(content_types.contains(&"tool_result"), "Should have tool_result blocks");
        assert!(content_types.contains(&"thinking"), "Should have thinking blocks");
    }

    #[test]
    fn test_parse_sets_has_tool_use() {
        let tmp = TempDir::new().unwrap();
        let path = write_fixture(tmp.path(), "test-uuid-1.jsonl", fixture_jsonl());
        let schema = IndexSchema::new();

        let docs = parse_jsonl_to_documents(&path, &schema, None);
        let session_doc = &docs[0];

        assert_eq!(
            session_doc.get_bool_val(schema.has_tool_use),
            Some(true),
            "Session with tool_use blocks should have has_tool_use=true"
        );
    }

    #[test]
    fn test_parse_aggregates_tokens() {
        let tmp = TempDir::new().unwrap();
        let path = write_fixture(tmp.path(), "test-uuid-1.jsonl", fixture_jsonl());
        let schema = IndexSchema::new();

        let docs = parse_jsonl_to_documents(&path, &schema, None);
        let session_doc = &docs[0];

        // input_tokens should be the LATEST: 250 (from last entry)
        assert_eq!(session_doc.get_u64_val(schema.input_tokens), Some(250));

        // output_tokens should be CUMULATIVE: 50 + 75 + 30 = 155
        assert_eq!(session_doc.get_u64_val(schema.output_tokens), Some(155));

        // total_tokens = input + output = 250 + 155 = 405
        assert_eq!(session_doc.get_u64_val(schema.total_tokens), Some(405));
    }

    #[test]
    fn test_parse_uses_index_metadata() {
        let tmp = TempDir::new().unwrap();
        let path = write_fixture(tmp.path(), "test-uuid-1.jsonl", fixture_jsonl());
        let schema = IndexSchema::new();

        let meta = SessionIndexEntry {
            session_id: "test-uuid-1".to_string(),
            full_path: None,
            first_prompt: Some("Custom first prompt".to_string()),
            summary: Some("Custom summary from index".to_string()),
            message_count: Some(42),
            created: None,
            modified: None,
            git_branch: None,
            project_path: None,
        };

        let docs = parse_jsonl_to_documents(&path, &schema, Some(&meta));
        let session_doc = &docs[0];

        assert_eq!(
            session_doc.get_str(schema.summary),
            Some("Custom summary from index"),
            "Should use summary from sessions-index.json"
        );
        assert_eq!(
            session_doc.get_str(schema.first_prompt),
            Some("Custom first prompt"),
            "Should use first_prompt from sessions-index.json"
        );
    }

    #[test]
    fn test_schema_version_mismatch_true_when_missing() {
        let tmp = TempDir::new().unwrap();
        assert!(
            schema_version_mismatch(tmp.path()),
            "Missing meta file should require reindex"
        );
    }

    #[test]
    fn test_schema_version_mismatch_false_when_current() {
        let tmp = TempDir::new().unwrap();
        let meta = IndexMeta {
            schema_version: SCHEMA_VERSION,
            indexed_at: "2026-02-18T12:00:00Z".to_string(),
            session_count: 100,
        };
        let json = serde_json::to_string(&meta).unwrap();
        fs::write(tmp.path().join("swarm-ui-meta.json"), json).unwrap();

        assert!(
            !schema_version_mismatch(tmp.path()),
            "Matching schema version should not require reindex"
        );
    }
}
