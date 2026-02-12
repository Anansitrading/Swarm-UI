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
    let mut cache_creation_tokens: u64 = 0;
    let mut cache_read_tokens: u64 = 0;
    let mut session_id = String::new();
    let mut git_branch = None;
    let mut cwd = None;
    let mut last_timestamp: u64 = 0;
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

        // Extract cwd from entry
        if cwd.is_none() {
            if let Some(c) = &entry.cwd {
                if !c.is_empty() {
                    cwd = Some(c.clone());
                }
            }
        }

        // Extract git branch from entry
        if git_branch.is_none() {
            if let Some(b) = &entry.git_branch {
                if !b.is_empty() {
                    git_branch = Some(b.clone());
                }
            }
        }

        // Extract timestamp
        if let Some(ts) = &entry.timestamp {
            if let Ok(t) = chrono_parse_unix(ts) {
                last_timestamp = t;
            }
        }

        // Skip non-message entries (progress, file-history-snapshot, etc.)
        let entry_type = entry.entry_type.as_deref().unwrap_or("");
        if entry_type != "user" && entry_type != "assistant" && entry_type != "tool" {
            continue;
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
                if let Some(ct) = usage.cache_creation_input_tokens {
                    cache_creation_tokens = ct; // Use latest
                }
                if let Some(cr) = usage.cache_read_input_tokens {
                    cache_read_tokens = cr; // Use latest
                }
            }

            // Status detection from message role and content
            if let Some(role) = &msg.role {
                match role.as_str() {
                    "assistant" => {
                        // Check content for tool_use blocks
                        if let Some(content) = &msg.content {
                            if let Some(arr) = content.as_array() {
                                for block in arr {
                                    if let Some(block_type) =
                                        block.get("type").and_then(|t| t.as_str())
                                    {
                                        match block_type {
                                            "tool_use" => {
                                                let name = block
                                                    .get("name")
                                                    .and_then(|n| n.as_str())
                                                    .unwrap_or("unknown")
                                                    .to_string();
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
    }

    // Determine final status based on timing
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // If last activity was >90 seconds ago, consider idle
    if last_timestamp > 0 && (now - last_timestamp) > 90 {
        if matches!(
            status,
            SessionStatus::Thinking | SessionStatus::ExecutingTool { .. }
        ) {
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

    // Use cwd if available, otherwise decode from directory name
    let project_path = if let Some(ref c) = cwd {
        c.clone()
    } else if project_dir.starts_with('-') {
        project_dir.replacen('-', "/", project_dir.len())
    } else {
        project_dir.to_string()
    };

    // Context = input_tokens + cache_creation + cache_read
    let context_tokens = input_tokens + cache_creation_tokens + cache_read_tokens;

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
        context_tokens,
        cache_creation_tokens,
        cache_read_tokens,
        git_branch,
        cwd,
    })
}

/// Parse ISO timestamp or unix timestamp string to epoch seconds
fn chrono_parse_unix(ts: &str) -> Result<u64, ()> {
    // Try ISO 8601: "2026-01-15T10:30:00.000Z"
    if ts.contains('T') {
        // Parse simple ISO format: YYYY-MM-DDTHH:MM:SS.sssZ
        // We just need epoch seconds, not full datetime parsing
        let date_part = ts.split('T').next().unwrap_or("");
        let time_part = ts
            .split('T')
            .nth(1)
            .unwrap_or("")
            .trim_end_matches('Z')
            .split('.')
            .next()
            .unwrap_or("");

        let date_parts: Vec<&str> = date_part.split('-').collect();
        let time_parts: Vec<&str> = time_part.split(':').collect();

        if date_parts.len() == 3 && time_parts.len() == 3 {
            let year: i64 = date_parts[0].parse().map_err(|_| ())?;
            let month: i64 = date_parts[1].parse().map_err(|_| ())?;
            let day: i64 = date_parts[2].parse().map_err(|_| ())?;
            let hour: i64 = time_parts[0].parse().map_err(|_| ())?;
            let min: i64 = time_parts[1].parse().map_err(|_| ())?;
            let sec: i64 = time_parts[2].parse().map_err(|_| ())?;

            // Simplified epoch calculation (approximate, good enough for relative timing)
            let days =
                (year - 1970) * 365 + (year - 1969) / 4 - (year - 1901) / 100 + (year - 1601) / 400;
            let month_days: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
            let mday = if month >= 1 && month <= 12 {
                month_days[(month - 1) as usize]
            } else {
                0
            };
            let is_leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            let leap_add = if is_leap && month > 2 { 1 } else { 0 };

            let total_days = days + mday + day - 1 + leap_add;
            let epoch = total_days * 86400 + hour * 3600 + min * 60 + sec;

            if epoch > 0 {
                return Ok(epoch as u64);
            }
        }
        return Err(());
    }

    // Try as plain number (milliseconds or seconds)
    if let Ok(num) = ts.parse::<u64>() {
        if num > 1_000_000_000_000 {
            return Ok(num / 1000); // milliseconds to seconds
        }
        return Ok(num);
    }
    Err(())
}

/// Extract conversation messages from a JSONL file for display
pub fn extract_conversation(path: &str) -> Result<Vec<ConversationMessage>, AppError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut messages = Vec::new();

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
            Err(_) => continue,
        };

        let entry_type = entry.entry_type.as_deref().unwrap_or("");
        if entry_type != "user" && entry_type != "assistant" {
            continue;
        }

        let timestamp = entry
            .timestamp
            .as_deref()
            .and_then(|ts| chrono_parse_unix(ts).ok());

        if let Some(msg) = &entry.message {
            let role = msg.role.as_deref().unwrap_or("unknown").to_string();

            if let Some(content) = &msg.content {
                if let Some(arr) = content.as_array() {
                    for block in arr {
                        let block_type = block
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("unknown");

                        match block_type {
                            "text" => {
                                let text = block
                                    .get("text")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if !text.is_empty() {
                                    messages.push(ConversationMessage {
                                        role: role.clone(),
                                        content_type: "text".to_string(),
                                        text,
                                        tool_name: None,
                                        timestamp,
                                    });
                                }
                            }
                            "tool_use" => {
                                let name = block
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                let input = block
                                    .get("input")
                                    .map(|i| serde_json::to_string_pretty(i).unwrap_or_default())
                                    .unwrap_or_default();
                                // Truncate long tool inputs
                                let display = if input.len() > 200 {
                                    format!("{}...", &input[..200])
                                } else {
                                    input
                                };
                                messages.push(ConversationMessage {
                                    role: role.clone(),
                                    content_type: "tool_use".to_string(),
                                    text: display,
                                    tool_name: Some(name),
                                    timestamp,
                                });
                            }
                            "tool_result" => {
                                let tool_id = block
                                    .get("tool_use_id")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let content_val = block.get("content");
                                let text =
                                    if let Some(arr2) = content_val.and_then(|c| c.as_array()) {
                                        arr2.iter()
                                            .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    } else if let Some(s) = content_val.and_then(|c| c.as_str()) {
                                        s.to_string()
                                    } else {
                                        String::new()
                                    };
                                // Truncate long results (char-safe boundary)
                                let display = if text.len() > 500 {
                                    let end = text
                                        .char_indices()
                                        .take_while(|(i, _)| *i <= 500)
                                        .last()
                                        .map(|(i, c)| i + c.len_utf8())
                                        .unwrap_or(0);
                                    format!("{}...", &text[..end])
                                } else {
                                    text
                                };
                                if !display.is_empty() {
                                    messages.push(ConversationMessage {
                                        role: "tool".to_string(),
                                        content_type: "tool_result".to_string(),
                                        text: display,
                                        tool_name: Some(tool_id),
                                        timestamp,
                                    });
                                }
                            }
                            "thinking" => {
                                let text = block
                                    .get("thinking")
                                    .or_else(|| block.get("text"))
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if !text.is_empty() {
                                    let display = if text.len() > 300 {
                                        let end = text
                                            .char_indices()
                                            .take_while(|(i, _)| *i <= 300)
                                            .last()
                                            .map(|(i, c)| i + c.len_utf8())
                                            .unwrap_or(0);
                                        format!("{}...", &text[..end])
                                    } else {
                                        text
                                    };
                                    messages.push(ConversationMessage {
                                        role: role.clone(),
                                        content_type: "thinking".to_string(),
                                        text: display,
                                        tool_name: None,
                                        timestamp,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                } else if let Some(text) = content.as_str() {
                    // Simple string content (older format)
                    if !text.is_empty() {
                        messages.push(ConversationMessage {
                            role: role.clone(),
                            content_type: "text".to_string(),
                            text: text.to_string(),
                            tool_name: None,
                            timestamp,
                        });
                    }
                }
            }
        }
    }

    Ok(messages)
}

/// Extract a compact text blob from a session for search indexing.
/// Returns user messages + assistant text blocks, truncated to ~4KB total.
pub fn extract_search_text(path: &str) -> Result<String, AppError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut parts: Vec<String> = Vec::new();
    let mut total_len = 0usize;
    const MAX_TOTAL: usize = 4096;

    for line in reader.lines() {
        if total_len >= MAX_TOTAL {
            break;
        }
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }
        let entry: JsonlEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let entry_type = entry.entry_type.as_deref().unwrap_or("");
        if entry_type != "user" && entry_type != "assistant" {
            continue;
        }
        if let Some(msg) = &entry.message {
            if let Some(content) = &msg.content {
                if let Some(arr) = content.as_array() {
                    for block in arr {
                        let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        if block_type == "text" {
                            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                let remaining = MAX_TOTAL.saturating_sub(total_len);
                                if remaining == 0 { break; }
                                let chunk = if text.len() > remaining {
                                    &text[..text.char_indices()
                                        .take_while(|(i, _)| *i <= remaining)
                                        .last()
                                        .map(|(i, c)| i + c.len_utf8())
                                        .unwrap_or(remaining)]
                                } else {
                                    text
                                };
                                total_len += chunk.len();
                                parts.push(chunk.to_string());
                            }
                        }
                    }
                } else if let Some(text) = content.as_str() {
                    let remaining = MAX_TOTAL.saturating_sub(total_len);
                    if remaining > 0 {
                        let chunk = if text.len() > remaining {
                            &text[..text.char_indices()
                                .take_while(|(i, _)| *i <= remaining)
                                .last()
                                .map(|(i, c)| i + c.len_utf8())
                                .unwrap_or(remaining.min(text.len()))]
                        } else {
                            text
                        };
                        total_len += chunk.len();
                        parts.push(chunk.to_string());
                    }
                }
            }
        }
    }

    Ok(parts.join(" "))
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
