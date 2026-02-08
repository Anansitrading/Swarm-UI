use crate::error::AppError;
use crate::parsers::session_types::{SessionInfo, SessionStatus};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

/// List all discovered Claude Code sessions from ~/.claude/projects/
#[tauri::command]
pub async fn list_sessions() -> Result<Vec<SessionInfo>, AppError> {
    let claude_dir = dirs::home_dir()
        .ok_or_else(|| AppError::Internal("No home dir".into()))?
        .join(".claude")
        .join("projects");

    if !claude_dir.exists() {
        return Ok(vec![]);
    }

    let mut sessions = Vec::new();

    for entry in fs::read_dir(&claude_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Decode path: -home-devuser-project -> /home/devuser/project
        let project_path = decode_claude_path(&dir_name);

        // Find .jsonl files in this project directory
        for file_entry in fs::read_dir(&path)? {
            let file_entry = file_entry?;
            let file_path = file_entry.path();
            if file_path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            let session_id = file_path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Get last modified time
            let metadata = fs::metadata(&file_path)?;
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            sessions.push(SessionInfo {
                id: session_id,
                project_path: project_path.clone(),
                encoded_path: dir_name.clone(),
                jsonl_path: file_path.to_string_lossy().to_string(),
                last_modified: modified,
                status: SessionStatus::Unknown,
                model: None,
                input_tokens: 0,
                output_tokens: 0,
                total_output_tokens: 0,
                git_branch: None,
            });
        }
    }

    // Sort by last modified descending
    sessions.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    Ok(sessions)
}

/// Get detailed session data by parsing the JSONL file
#[tauri::command]
pub async fn get_session_detail(jsonl_path: String) -> Result<SessionInfo, AppError> {
    use crate::parsers::jsonl_parser;
    jsonl_parser::parse_session_file(&jsonl_path)
}

/// Decode Claude's path encoding: dashes become path separators
/// "-home-devuser-Kijko-MVP" -> "/home/devuser/Kijko-MVP"
fn decode_claude_path(encoded: &str) -> String {
    // The encoding replaces / with - but also - in names stays as -
    // Strategy: split on -, rebuild as path, check if it exists
    // Simple approach: replace leading -home- pattern
    if encoded.starts_with('-') {
        let path = encoded.replacen('-', "/", encoded.len());
        // Clean up: the encoding is literal replacement of / with -
        // So -home-devuser-project = /home/devuser/project
        return path;
    }
    encoded.to_string()
}
