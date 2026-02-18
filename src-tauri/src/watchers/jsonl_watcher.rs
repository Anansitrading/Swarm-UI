use crate::error::AppError;
use crate::state::AppState;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};

/// Start watching ~/.claude/projects/ for JSONL file changes.
/// Emits "session:updated" events when session files are modified.
/// Uses debouncing + cached parsing for efficiency.
#[tauri::command]
pub async fn start_session_watcher(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let claude_dir = dirs::home_dir()
        .ok_or_else(|| AppError::Internal("No home dir".into()))?
        .join(".claude")
        .join("projects");

    if !claude_dir.exists() {
        return Err(AppError::NotFound(
            "~/.claude/projects/ not found".to_string(),
        ));
    }

    // TODO: Replace with Tantivy incremental watcher once search module is wired up.
    let _ = &state; // silence unused warning until Tantivy wired up

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

        // Debounce: collect changed paths and flush every 1 second
        let debounce_interval = Duration::from_secs(1);
        let mut pending_paths: HashSet<String> = HashSet::new();
        let mut last_flush = Instant::now();

        loop {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(event) => {
                    for path in &event.paths {
                        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                            continue;
                        }
                        pending_paths.insert(path.to_string_lossy().to_string());
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Fall through to flush check
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }

            // Flush pending paths if debounce interval has passed
            if !pending_paths.is_empty() && last_flush.elapsed() >= debounce_interval {
                for path_str in pending_paths.drain() {
                    // TODO: Replace with Tantivy incremental indexing.
                    match crate::parsers::jsonl_parser::parse_session_tail(&path_str) {
                        Ok(info) => {
                            let _ = app.emit("session:updated", &info);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse {path_str}: {e}");
                        }
                    }
                }
                last_flush = Instant::now();
            }
        }
    });

    Ok(())
}
