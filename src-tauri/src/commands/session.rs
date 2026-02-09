use crate::error::AppError;
use crate::parsers::session_types::{ConversationMessage, SessionInfo, SessionStatus};
use crate::state::{AppState, PtyInfo, PtyInstance};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::fs;
use std::io::Read;
use std::thread;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

/// List all discovered Claude Code sessions from ~/.claude/projects/
/// Parses JSONL files for live status detection.
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

        // Find .jsonl files in this project directory
        for file_entry in fs::read_dir(&path)? {
            let file_entry = file_entry?;
            let file_path = file_entry.path();
            if file_path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            let path_str = file_path.to_string_lossy().to_string();

            // Parse the JSONL file for full session info including status
            match crate::parsers::jsonl_parser::parse_session_file(&path_str) {
                Ok(info) => {
                    sessions.push(info);
                }
                Err(e) => {
                    // Fallback: create basic entry from filesystem metadata
                    let dir_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                    let session_id = file_path
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                    let modified = fs::metadata(&file_path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    tracing::warn!("Failed to parse {path_str}: {e}");
                    sessions.push(SessionInfo {
                        id: session_id,
                        project_path: decode_claude_path(&dir_name),
                        encoded_path: dir_name,
                        jsonl_path: path_str,
                        last_modified: modified,
                        status: SessionStatus::Unknown,
                        model: None,
                        input_tokens: 0,
                        output_tokens: 0,
                        total_output_tokens: 0,
                        context_tokens: 0,
                        cache_creation_tokens: 0,
                        cache_read_tokens: 0,
                        git_branch: None,
                        cwd: None,
                    });
                }
            }
        }
    }

    // Sort by last modified descending
    sessions.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    Ok(sessions)
}

/// Get detailed session data by parsing the JSONL file
#[tauri::command]
pub async fn get_session_detail(jsonl_path: String) -> Result<SessionInfo, AppError> {
    crate::parsers::jsonl_parser::parse_session_file(&jsonl_path)
}

/// Get conversation messages for display in the UI
#[tauri::command]
pub async fn get_conversation(jsonl_path: String) -> Result<Vec<ConversationMessage>, AppError> {
    crate::parsers::jsonl_parser::extract_conversation(&jsonl_path)
}

/// Inject a steering message into a Claude Code session by resuming it in a PTY.
/// For idle/waiting sessions, spawns `claude --resume <id>` and sends the message.
/// Returns PtyInfo so the frontend can track the resumed session output.
#[tauri::command]
pub async fn inject_session_message(
    session_id: String,
    message: String,
    cwd: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<PtyInfo, AppError> {
    let id = Uuid::new_v4().to_string();
    let cols: u16 = 120;
    let rows: u16 = 40;

    let pty_system = NativePtySystem::default();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| AppError::Pty(e.to_string()))?;

    // Build the claude --resume command
    let mut cmd = CommandBuilder::new("claude");
    cmd.arg("--resume");
    cmd.arg(&session_id);
    cmd.arg("--dangerously-skip-permissions");
    cmd.cwd(&cwd);
    cmd.env("TERM", "xterm-256color");

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| AppError::Pty(e.to_string()))?;

    let pid = child.process_id().unwrap_or(0);

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| AppError::Pty(e.to_string()))?;

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| AppError::Pty(e.to_string()))?;

    let info = PtyInfo {
        id: id.clone(),
        pid,
        cols,
        rows,
    };

    // Store the PTY instance
    {
        let mut ptys = state.ptys.lock().unwrap();
        ptys.insert(
            id.clone(),
            PtyInstance {
                id: id.clone(),
                master: pair.master,
                writer,
                child,
                cols,
                rows,
            },
        );
    }

    // Clone app handle before moving into threads
    let app_for_inject = app.clone();

    // Spawn reader thread to stream PTY output as events
    let pty_id = id.clone();
    let event_name = format!("pty:data:{}", pty_id);
    let exit_event = format!("pty:exit:{}", pty_id);
    thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = crate::commands::pty::base64_encode_pub(&buf[..n]);
                    let _ = app.emit(&event_name, data);
                }
                Err(_) => break,
            }
        }
        let _ = app.emit(&exit_event, ());
    });

    // After a delay for Claude to load, emit an event telling the frontend
    // to send the steering message via pty_write. This avoids ownership issues
    // with the writer (which is stored in PtyInstance).
    let steering_event = format!("pty:inject:{}", id);
    thread::spawn(move || {
        thread::sleep(std::time::Duration::from_millis(3000));
        let _ = app_for_inject.emit(&steering_event, message);
    });

    Ok(info)
}

/// Decode Claude's path encoding: dashes become path separators
/// "-home-devuser-Kijko-MVP" -> "/home/devuser/Kijko-MVP"
///
/// The encoding replaces each / with - so the trick is to reconstruct
/// the path by trying known prefixes. The actual pattern is that the
/// encoded path starts with - and each directory separator was replaced with -.
fn decode_claude_path(encoded: &str) -> String {
    if !encoded.starts_with('-') {
        return encoded.to_string();
    }

    // Remove leading dash, split by -, rebuild by checking which segments
    // form valid paths
    let without_dash = &encoded[1..];
    let parts: Vec<&str> = without_dash.split('-').collect();

    // Try to greedily reconstruct the path
    let mut result = String::from("/");
    let mut i = 0;
    while i < parts.len() {
        if i > 0 {
            // Try joining remaining parts with - first (to handle names with dashes)
            // then with / to find the longest valid directory path
            let mut found = false;
            // Try treating this part as a new path segment
            let test_path = format!("{}{}", result, parts[i]);
            if std::path::Path::new(&test_path).exists() {
                result = test_path;
                if i < parts.len() - 1 {
                    result.push('/');
                }
                found = true;
            }
            if !found {
                // Append with dash (part of the name)
                // Remove trailing /
                if result.ends_with('/') {
                    result.pop();
                }
                result.push('-');
                result.push_str(parts[i]);
                if i < parts.len() - 1 {
                    // Check if this directory exists to decide separator
                    if std::path::Path::new(&result).is_dir() {
                        result.push('/');
                    }
                }
            }
        } else {
            result.push_str(parts[i]);
            if std::path::Path::new(&result).is_dir() && i < parts.len() - 1 {
                result.push('/');
            }
        }
        i += 1;
    }

    result
}
