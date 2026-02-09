mod commands;
mod error;
mod parsers;
mod sprite;
mod sprites_api;
mod sprites_ws;
mod state;
mod watchers;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // PTY commands
            commands::pty::pty_spawn,
            commands::pty::pty_write,
            commands::pty::pty_resize,
            commands::pty::pty_kill,
            commands::pty::pty_list,
            // Session commands
            commands::session::list_sessions,
            commands::session::get_session_detail,
            commands::session::get_conversation,
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
            // Filesystem commands
            commands::filesystem::read_file,
            commands::filesystem::read_file_range,
            // Watcher commands
            watchers::jsonl_watcher::start_session_watcher,
            watchers::pool_watcher::get_bot_pool_state,
            watchers::pool_watcher::start_pool_watcher,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Swarm-UI");
}
