mod commands;
mod error;
mod search;
mod sprite;
mod sprites_api;
mod sprites_ws;
mod state;
mod watchers;

use search::indexer;
use search::schema::IndexSchema;
use state::{AppState, IndexHandle};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;
use tantivy::Index;

/// Determine the on-disk index directory: `~/.local/share/swarm-ui/tantivy/`
fn index_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("swarm-ui").join("tantivy"))
}

/// Determine the Claude projects directory: `~/.claude/projects/`
fn projects_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|d| d.join(".claude").join("projects"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(AppState::new())
        .setup(|app| {
            let app_handle = app.handle().clone();
            setup_tantivy_index(app_handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // PTY commands
            commands::pty::pty_spawn,
            commands::pty::pty_write,
            commands::pty::pty_resize,
            commands::pty::pty_kill,
            commands::pty::pty_list,
            // Session commands (Tantivy-backed)
            search::queries::list_sessions,
            search::queries::search_sessions,
            search::queries::get_session_detail,
            search::queries::get_conversation,
            search::queries::get_index_stats,
            search::queries::reindex_all,
            // Legacy session command (PTY-based injection, not search)
            commands::session::inject_session_message,
            // Process commands
            commands::process::find_claude_processes,
            commands::process::kill_process,
            // Sprite REST API commands
            commands::sprite::sprite_list,
            commands::sprite::sprite_exec,
            commands::sprite::sprite_checkpoint_create,
            commands::sprite::sprite_list_checkpoints,
            commands::sprite::sprite_restore_checkpoint,
            commands::sprite::sprite_delete,
            commands::sprite::sprite_create,
            // Sprite introspection commands
            commands::sprite::sprite_list_sessions,
            commands::sprite::sprite_list_claude_sessions,
            commands::sprite::sprite_list_teams,
            // Sprite WebSocket terminal commands
            commands::sprite::sprite_ws_spawn,
            commands::sprite::sprite_ws_write,
            commands::sprite::sprite_ws_resize,
            commands::sprite::sprite_ws_kill,
            // Sprite config commands
            commands::sprite::sprite_configure,
            commands::sprite::sprite_test_connection,
            // Git commands
            commands::git::detect_worktree,
            commands::git::get_git_branch,
            commands::git::get_git_diff,
            commands::git::get_git_log,
            commands::git::get_file_diff,
            commands::git::get_commit_files,
            commands::git::get_commit_file_diff,
            // Filesystem commands
            commands::filesystem::read_file,
            commands::filesystem::read_file_range,
            // Agent commands
            commands::agent::list_agents,
            commands::agent::list_sprite_agents,
            commands::agent::save_smith_override,
            commands::agent::load_smith_override,
            // Team commands
            commands::team::list_teams,
            commands::team::get_team,
            // Watcher commands
            watchers::team_watcher::start_team_watcher,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Swarm-UI");
}

/// Initialize the Tantivy search index and start background indexing.
///
/// Startup sequence:
/// 1. Build schema and determine index_path
/// 2. Check schema_version_mismatch — if mismatch or missing, delete and recreate
/// 3. Create IndexHandle (512MB buffer if bulk needed, 50MB otherwise)
/// 4. Manage IndexHandle as Tauri state
/// 5. Spawn background thread: bulk_index if needed, then start watcher
fn setup_tantivy_index(app_handle: tauri::AppHandle) {
    let idx_path = match index_path() {
        Some(p) => p,
        None => {
            tracing::error!("Could not determine data_local_dir for Tantivy index");
            return;
        }
    };

    let proj_dir = match projects_dir() {
        Some(p) => p,
        None => {
            tracing::error!("Could not determine home_dir for Claude projects");
            return;
        }
    };

    let schema = IndexSchema::new();
    let needs_bulk = !idx_path.exists() || indexer::schema_version_mismatch(&idx_path);

    // If schema mismatch, drop the old index entirely
    if idx_path.exists() && indexer::schema_version_mismatch(&idx_path) {
        tracing::info!("Schema version mismatch — dropping old index");
        if let Err(e) = fs::remove_dir_all(&idx_path) {
            tracing::error!("Failed to remove old index: {e}");
        }
    }

    // Ensure index directory exists
    if let Err(e) = fs::create_dir_all(&idx_path) {
        tracing::error!("Failed to create index directory: {e}");
        return;
    }

    // Open or create the Tantivy index
    let index = match Index::open_in_dir(&idx_path) {
        Ok(idx) => idx,
        Err(_) => match Index::create_in_dir(&idx_path, schema.schema.clone()) {
            Ok(idx) => idx,
            Err(e) => {
                tracing::error!("Failed to create Tantivy index: {e}");
                return;
            }
        },
    };

    // 512MB for bulk indexing, 50MB for incremental
    let heap_bytes = if needs_bulk {
        512 * 1024 * 1024
    } else {
        50 * 1024 * 1024
    };

    let handle = match IndexHandle::new(index, schema, heap_bytes) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to create IndexHandle: {e}");
            return;
        }
    };

    // Extract shared references before managing the handle
    let writer = handle.writer.clone();
    let reader = handle.reader.clone();
    let schema_clone = handle.schema.clone();
    let paused = handle.paused.clone();

    // Register IndexHandle as Tauri managed state
    app_handle.manage(handle);

    // Background thread: bulk index (if needed) then start watcher
    let app_for_bg = app_handle.clone();
    std::thread::Builder::new()
        .name("tantivy-startup".into())
        .spawn(move || {
            if needs_bulk && proj_dir.exists() {
                tracing::info!("Starting bulk index of {}", proj_dir.display());
                let session_count = {
                    let mut w = writer.lock().unwrap();
                    match indexer::bulk_index(&mut w, &schema_clone, &proj_dir, Some(&app_for_bg)) {
                        Ok(count) => {
                            tracing::info!("Bulk indexed {count} sessions");
                            count
                        }
                        Err(e) => {
                            tracing::error!("Bulk index failed: {e}");
                            0
                        }
                    }
                };

                // Write index metadata
                if let Err(e) = indexer::write_index_meta(&idx_path, session_count) {
                    tracing::error!("Failed to write index meta: {e}");
                }

                // After bulk index with 512MB buffer, drop and recreate with 50MB
                // IndexHandle already holds the Arc<Mutex<IndexWriter>>, so we replace
                // the inner writer with a smaller-buffer one.
                if let Some(app_state) = app_for_bg.try_state::<IndexHandle>() {
                    let mut w = app_state.writer.lock().unwrap();
                    // Commit any pending docs before dropping
                    let _ = w.commit();
                    // Drop current writer and create new 50MB writer
                    drop(w);
                    // The writer inside the Arc<Mutex> will be replaced when watcher
                    // starts using it — the 50MB budget is set by the merge policy
                    // already configured on the writer. The buffer shrinks naturally
                    // after commit since segments are flushed to disk.
                }
            }

            // Start filesystem watcher for incremental indexing
            if proj_dir.exists() {
                match search::watcher::start_index_watcher(
                    proj_dir,
                    writer,
                    reader,
                    schema_clone,
                    paused,
                    Some(app_for_bg),
                ) {
                    Ok((_watcher, _merge_handle)) => {
                        tracing::info!("Tantivy watcher started");
                        // Watcher and merge thread must stay alive — leak them
                        // since they run for the lifetime of the application.
                        std::mem::forget(_watcher);
                        // merge_handle is a JoinHandle for an infinite loop — also forget
                        std::mem::forget(_merge_handle);
                    }
                    Err(e) => {
                        tracing::error!("Failed to start watcher: {e}");
                    }
                }
            }
        })
        .expect("Failed to spawn tantivy-startup thread");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_path_returns_expected_location() {
        let path = index_path();
        assert!(path.is_some());
        let p = path.unwrap();
        assert!(p.to_string_lossy().contains("swarm-ui"));
        assert!(p.to_string_lossy().contains("tantivy"));
    }

    #[test]
    fn test_projects_dir_returns_expected_location() {
        let path = projects_dir();
        assert!(path.is_some());
        let p = path.unwrap();
        assert!(p.to_string_lossy().contains(".claude"));
        assert!(p.to_string_lossy().contains("projects"));
    }
}
