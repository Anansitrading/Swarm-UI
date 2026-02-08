mod commands;
mod error;
mod parsers;
mod sprite;
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
            // Process commands
            commands::process::find_claude_processes,
            commands::process::kill_process,
            // Sprite commands
            commands::sprite::sprite_list,
            commands::sprite::sprite_exec,
            commands::sprite::sprite_checkpoint_create,
            // Git commands
            commands::git::detect_worktree,
            commands::git::get_git_branch,
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
