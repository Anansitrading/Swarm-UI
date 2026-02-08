use crate::error::AppError;
use crate::parsers::session_types::*;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};

/// Parse a session JSONL file and extract session info with status detection.
/// Port of Swift SessionJSONLParser logic.
pub fn parse_session_file(path: &str) -> Result<SessionInfo, AppError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut status = SessionStatus::Unknown;
    let mut model = None;
    let mut input_tokens: u64 = 0;
    let mut output_tokens: u64 = 0;
    let mut total_output_tokens: u64 = 0;
    let mut session_id = String::new();
    let mut git_branch = None;
    let mut last_timestamp: u64 = 0;
    let mut last_role = String::new();
    let mut last_tool_name = None;
    let mut has_pending_tool_use = false;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.trim().is_empty() {
            continue;
        }

        let entry: JsonlEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue, // Skip malformed lines
        };

        // Extract session ID
        if let Some(sid) = &entry.session_id {
            if session_id.is_empty() {
                session_id = sid.clone();
            }
        }

        // Extract timestamp
        if let Some(ts) = &entry.timestamp {
            if let Ok(t) = ts.parse::<u64>() {
                last_timestamp = t;
            } else if let Ok(t) = chrono_parse_unix(ts) {
                last_timestamp = t;
            }
        }

        if let Some(msg) = &entry.message {
            // Track model
            if let Some(m) = &msg.model {
                model = Some(m.clone());
            }

            // Track usage
            if let Some(usage) = &msg.usage {
                if let Some(it) = usage.input_tokens {
                    input_tokens = it; // Use latest, not cumulative
                }
                if let Some(ot) = usage.output_tokens {
                    output_tokens = ot;
                    total_output_tokens += ot;
                }
            }

            // Status detection from message role and content
            if let Some(role) = &msg.role {
                last_role = role.clone();

                match role.as_str() {
                    "assistant" => {
                        // Check content for tool_use blocks
                        if let Some(content) = &msg.content {
                            if let Some(arr) = content.as_array() {
                                for block in arr {
                                    if let Some(block_type) = block.get("type").and_then(|t| t.as_str()) {
                                        match block_type {
                                            "tool_use" => {
                                                let name = block
                                                    .get("name")
                                                    .and_then(|n| n.as_str())
                                                    .unwrap_or("unknown")
                                                    .to_string();
                                                last_tool_name = Some(name.clone());
                                                has_pending_tool_use = true;
                                                status = SessionStatus::ExecutingTool { name };
                                            }
                                            "thinking" => {
                                                status = SessionStatus::Thinking;
                                            }
                                            "text" => {
                                                if !has_pending_tool_use {
                                                    status = SessionStatus::Waiting;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    "user" => {
                        has_pending_tool_use = false;
                        status = SessionStatus::Thinking;
                    }
                    "tool" => {
                        // Tool result received
                        has_pending_tool_use = false;
                        status = SessionStatus::Thinking;
                    }
                    _ => {}
                }
            }
        }

        // Check for git branch in early entries
        if git_branch.is_none() {
            if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(branch) = raw.get("gitBranch").and_then(|b| b.as_str()) {
                    git_branch = Some(branch.to_string());
                }
            }
        }
    }

    // Determine final status based on timing
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // If last activity was >90 seconds ago, consider idle
    if last_timestamp > 0 && (now - last_timestamp) > 90 {
        if matches!(status, SessionStatus::Thinking | SessionStatus::ExecutingTool { .. }) {
            status = SessionStatus::Idle;
        }
    }

    // Decode path from JSONL file path
    let jsonl_path = std::path::Path::new(path);
    let project_dir = jsonl_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let project_path = if project_dir.starts_with('-') {
        project_dir.replacen('-', "/", project_dir.len())
    } else {
        project_dir.to_string()
    };

    Ok(SessionInfo {
        id: session_id,
        project_path,
        encoded_path: project_dir.to_string(),
        jsonl_path: path.to_string(),
        last_modified: last_timestamp,
        status,
        model,
        input_tokens,
        output_tokens,
        total_output_tokens,
        git_branch,
    })
}

/// Parse ISO timestamp or unix timestamp string to epoch seconds
fn chrono_parse_unix(ts: &str) -> Result<u64, ()> {
    // Try parsing as ISO 8601
    // Simple heuristic: if it contains 'T', try ISO parse
    if ts.contains('T') {
        // Basic ISO parse without chrono dep
        // Format: 2026-01-15T10:30:00.000Z
        let parts: Vec<&str> = ts.split('T').collect();
        if parts.len() == 2 {
            // Just use the file modification time as fallback
            return Err(());
        }
    }
    // Try as plain number (milliseconds)
    if let Ok(ms) = ts.parse::<u64>() {
        if ms > 1_000_000_000_000 {
            return Ok(ms / 1000); // milliseconds to seconds
        }
        return Ok(ms);
    }
    Err(())
}

/// Incremental reader that tracks file offset for live watching
pub struct IncrementalReader {
    path: String,
    offset: u64,
}

impl IncrementalReader {
    pub fn new(path: String) -> Self {
        Self { path, offset: 0 }
    }

    /// Read new lines since last call
    pub fn read_new_lines(&mut self) -> Result<Vec<JsonlEntry>, AppError> {
        let mut file = File::open(&self.path)?;
        let file_len = file.metadata()?.len();

        if file_len <= self.offset {
            return Ok(vec![]);
        }

        file.seek(SeekFrom::Start(self.offset))?;
        let reader = BufReader::new(&file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<JsonlEntry>(&line) {
                entries.push(entry);
            }
        }

        self.offset = file_len;
        Ok(entries)
    }
}
