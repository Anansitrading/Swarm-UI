use crate::error::AppError;
use crate::parsers::jsonl_parser::IncrementalReader;
use crate::parsers::session_types::SessionInfo;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Start watching ~/.claude/projects/ for JSONL file changes.
/// Emits "session:updated" events when session files are modified.
#[tauri::command]
pub async fn start_session_watcher(app: AppHandle) -> Result<(), AppError> {
    let claude_dir = dirs::home_dir()
        .ok_or_else(|| AppError::Internal("No home dir".into()))?
        .join(".claude")
        .join("projects");

    if !claude_dir.exists() {
        return Err(AppError::NotFound(
            "~/.claude/projects/ not found".to_string(),
        ));
    }

    // Spawn watcher in a background thread
    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel();

        let mut watcher = match RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(2)),
        ) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("Failed to create watcher: {e}");
                return;
            }
        };

        if let Err(e) = watcher.watch(&claude_dir, RecursiveMode::Recursive) {
            tracing::error!("Failed to watch {}: {e}", claude_dir.display());
            return;
        }

        tracing::info!("Watching {} for JSONL changes", claude_dir.display());

        let mut readers: HashMap<String, IncrementalReader> = HashMap::new();

        loop {
            match rx.recv_timeout(Duration::from_secs(5)) {
                Ok(event) => {
                    for path in &event.paths {
                        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                            continue;
                        }

                        let path_str = path.to_string_lossy().to_string();

                        // Parse the updated session file
                        match crate::parsers::jsonl_parser::parse_session_file(&path_str) {
                            Ok(info) => {
                                let _ = app.emit("session:updated", &info);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse {path_str}: {e}");
                            }
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Periodic check - could emit heartbeat
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Ok(())
}
