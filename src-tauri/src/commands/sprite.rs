use crate::error::AppError;
use crate::sprites_api;
use crate::state::{AppState, PtyInfo};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

// ==========================================
// Sprites CRUD
// ==========================================

/// List all sprites via REST API
#[tauri::command]
pub async fn sprite_list(
    state: State<'_, AppState>,
) -> Result<Vec<sprites_api::SpriteInfo>, AppError> {
    let client = state.get_sprites_client()?;
    client.list_sprites().await
}

/// Get sprite details
#[tauri::command]
pub async fn sprite_get(
    name: String,
    state: State<'_, AppState>,
) -> Result<sprites_api::SpriteDetail, AppError> {
    let client = state.get_sprites_client()?;
    client.get_sprite(&name).await
}

/// Create a new sprite
#[tauri::command]
pub async fn sprite_create(
    name: String,
    state: State<'_, AppState>,
) -> Result<sprites_api::SpriteInfo, AppError> {
    let client = state.get_sprites_client()?;
    client.create_sprite(&name).await
}

/// Update sprite settings (url_settings.auth)
#[tauri::command]
pub async fn sprite_update(
    name: String,
    url_auth: String,
    state: State<'_, AppState>,
) -> Result<sprites_api::SpriteDetail, AppError> {
    let client = state.get_sprites_client()?;
    client.update_sprite(&name, &url_auth).await
}

/// Delete a sprite
#[tauri::command]
pub async fn sprite_delete(name: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let client = state.get_sprites_client()?;
    client.delete_sprite(&name).await
}

// ==========================================
// Exec
// ==========================================

/// Execute a command on a sprite via HTTP POST (non-interactive)
#[tauri::command]
pub async fn sprite_exec(
    name: String,
    command: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let client = state.get_sprites_client()?;
    let result = client.exec_http(&name, &command).await?;

    let mut output = result.stdout;
    if !result.stderr.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&result.stderr);
    }
    Ok(output)
}

/// Execute a shell command on a sprite via query-param API (returns raw output)
#[tauri::command]
pub async fn sprite_exec_command(
    name: String,
    command: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let client = state.get_sprites_client()?;
    client.exec_command(&name, &command).await
}

/// List exec sessions (real API)
#[tauri::command]
pub async fn sprite_list_exec_sessions(
    name: String,
    state: State<'_, AppState>,
) -> Result<Vec<sprites_api::ExecSession>, AppError> {
    let client = state.get_sprites_client()?;
    client.list_exec_sessions(&name).await
}

/// Kill an exec session via NDJSON streaming
#[tauri::command]
pub async fn sprite_kill_exec_session(
    name: String,
    session_id: String,
    signal: Option<String>,
    on_event: Channel<sprites_api::ExecKillEvent>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let client = state.get_sprites_client()?;
    let sig = signal.as_deref().unwrap_or("SIGTERM");
    let resp = client
        .kill_exec_session_stream(&name, &session_id, sig)
        .await?;
    sprites_api::pipe_ndjson_stream(resp, &on_event, sprites_api::ExecKillEvent::is_terminal).await
}

// ==========================================
// Checkpoints — NDJSON streaming via Channel
// ==========================================

/// Create a checkpoint with NDJSON streaming progress
#[tauri::command]
pub async fn sprite_checkpoint_create(
    name: String,
    comment: Option<String>,
    on_event: Channel<sprites_api::StreamEvent>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let client = state.get_sprites_client()?;
    let resp = client
        .create_checkpoint_stream(&name, comment.as_deref())
        .await?;
    sprites_api::pipe_ndjson_stream(resp, &on_event, sprites_api::StreamEvent::is_terminal).await
}

/// List checkpoints for a sprite
#[tauri::command]
pub async fn sprite_list_checkpoints(
    name: String,
    state: State<'_, AppState>,
) -> Result<Vec<sprites_api::Checkpoint>, AppError> {
    let client = state.get_sprites_client()?;
    client.list_checkpoints(&name).await
}

/// Restore a checkpoint with NDJSON streaming progress
#[tauri::command]
pub async fn sprite_restore_checkpoint(
    name: String,
    checkpoint_id: String,
    on_event: Channel<sprites_api::StreamEvent>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let client = state.get_sprites_client()?;
    let resp = client
        .restore_checkpoint_stream(&name, &checkpoint_id)
        .await?;
    sprites_api::pipe_ndjson_stream(resp, &on_event, sprites_api::StreamEvent::is_terminal).await
}

// ==========================================
// Services — NDJSON streaming via Channel
// ==========================================

/// List services for a sprite
#[tauri::command]
pub async fn sprite_list_services(
    name: String,
    state: State<'_, AppState>,
) -> Result<Vec<sprites_api::Service>, AppError> {
    let client = state.get_sprites_client()?;
    client.list_services(&name).await
}

/// Start a service with NDJSON streaming progress
#[tauri::command]
pub async fn sprite_start_service(
    name: String,
    service_name: String,
    on_event: Channel<sprites_api::ServiceStreamEvent>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let client = state.get_sprites_client()?;
    let resp = client.start_service_stream(&name, &service_name).await?;
    sprites_api::pipe_ndjson_stream(resp, &on_event, sprites_api::ServiceStreamEvent::is_terminal)
        .await
}

/// Stop a service with NDJSON streaming progress
#[tauri::command]
pub async fn sprite_stop_service(
    name: String,
    service_name: String,
    on_event: Channel<sprites_api::ServiceStreamEvent>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let client = state.get_sprites_client()?;
    let resp = client.stop_service_stream(&name, &service_name).await?;
    sprites_api::pipe_ndjson_stream(resp, &on_event, sprites_api::ServiceStreamEvent::is_terminal)
        .await
}

/// Get service logs with NDJSON streaming
#[tauri::command]
pub async fn sprite_get_service_logs(
    name: String,
    service_name: String,
    lines: Option<u32>,
    on_event: Channel<sprites_api::ServiceStreamEvent>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let client = state.get_sprites_client()?;
    let resp = client
        .get_service_logs_stream(&name, &service_name, lines)
        .await?;
    sprites_api::pipe_ndjson_stream(resp, &on_event, sprites_api::ServiceStreamEvent::is_terminal)
        .await
}

// ==========================================
// Sprite introspection (shell commands on VM)
// ==========================================

#[derive(Debug, Serialize)]
pub struct SpriteSessionInfo {
    pub pid: String,
    pub command: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct SpriteClaudeSessionInfo {
    pub session_id: String,
    pub project_dir: String,
    pub jsonl_path: String,
}

#[derive(Debug, Serialize)]
pub struct SpriteTeamInfo {
    pub name: String,
    pub member_count: u32,
}

/// List exec sessions running on a sprite (via ps aux on the VM)
#[tauri::command]
pub async fn sprite_list_sessions(
    name: String,
    state: State<'_, AppState>,
) -> Result<Vec<SpriteSessionInfo>, AppError> {
    let client = state.get_sprites_client()?;
    let result = client
        .exec_http(
            &name,
            "ps aux --no-headers 2>/dev/null | grep -v 'grep' | head -20 || echo ''",
        )
        .await?;
    let mut sessions = Vec::new();
    for line in result.stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(11, char::is_whitespace).collect();
        if parts.len() >= 11 {
            let cmd = parts[10].to_string();
            let pid = parts[1].to_string();
            if cmd.contains("claude") || cmd.contains("node") || cmd.contains("bash") {
                sessions.push(SpriteSessionInfo {
                    pid,
                    command: cmd,
                    status: "running".to_string(),
                });
            }
        }
    }
    Ok(sessions)
}

/// List Claude Code sessions found on a sprite
#[tauri::command]
pub async fn sprite_list_claude_sessions(
    name: String,
    state: State<'_, AppState>,
) -> Result<Vec<SpriteClaudeSessionInfo>, AppError> {
    let client = state.get_sprites_client()?;
    let result = client
        .exec_http(
            &name,
            "find ~/.claude/projects -name '*.jsonl' -printf '%T@ %p\\n' 2>/dev/null | sort -rn | head -20",
        )
        .await?;

    let mut sessions = Vec::new();
    for line in result.stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() == 2 {
            let path = parts[1];
            if let Some(fname) = path.rsplit('/').next() {
                let session_id = fname.trim_end_matches(".jsonl").to_string();
                let project_dir = path.rsplit('/').nth(1).unwrap_or("").to_string();
                sessions.push(SpriteClaudeSessionInfo {
                    session_id,
                    project_dir,
                    jsonl_path: path.to_string(),
                });
            }
        }
    }
    Ok(sessions)
}

/// List Claude agent teams on a sprite
#[tauri::command]
pub async fn sprite_list_teams(
    name: String,
    state: State<'_, AppState>,
) -> Result<Vec<SpriteTeamInfo>, AppError> {
    let client = state.get_sprites_client()?;
    let result = client
        .exec_http(
            &name,
            "for d in ~/.claude/teams/*/; do [ -f \"$d/config.json\" ] && echo \"$(basename $d)|$(cat $d/config.json 2>/dev/null)\"; done 2>/dev/null",
        )
        .await?;

    let mut teams = Vec::new();
    for line in result.stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(2, '|').collect();
        if parts.len() == 2 {
            let team_name = parts[0].to_string();
            let member_count = parts[1].matches("\"name\"").count() as u32;
            teams.push(SpriteTeamInfo {
                name: team_name,
                member_count,
            });
        }
    }
    Ok(teams)
}

// ==========================================
// WebSocket terminal
// ==========================================

/// Spawn an interactive WebSocket terminal to a sprite
#[tauri::command]
pub async fn sprite_ws_spawn(
    sprite_name: String,
    cols: Option<u16>,
    rows: Option<u16>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<PtyInfo, AppError> {
    tracing::info!("sprite_ws_spawn called for '{sprite_name}'");
    let client = state.get_sprites_client()?;
    let c = cols.unwrap_or(80);
    let r = rows.unwrap_or(24);
    let ws_url = client.ws_exec_url(&sprite_name, c, r);
    let token = client.token().to_string();

    crate::sprites_ws::sprite_ws_connect(&sprite_name, &ws_url, &token, c, r, app, &state.ws_state)
        .await
}

/// Write to a sprite WebSocket terminal
#[tauri::command]
pub async fn sprite_ws_write(
    id: String,
    data: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &data)
        .map_err(|e| AppError::Internal(format!("base64 decode failed: {e}")))?;

    crate::sprites_ws::ws_write(&id, &decoded, &state.ws_state).await
}

/// Resize a sprite WebSocket terminal
#[tauri::command]
pub async fn sprite_ws_resize(
    id: String,
    cols: u16,
    rows: u16,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    crate::sprites_ws::ws_resize(&id, cols, rows, &state.ws_state).await
}

/// Kill a sprite WebSocket terminal
#[tauri::command]
pub async fn sprite_ws_kill(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    crate::sprites_ws::ws_kill(&id, &state.ws_state).await
}

// ==========================================
// Claude provisioning
// ==========================================

/// Push local ~/.claude/.credentials.json to a sprite so Claude Code can authenticate
#[tauri::command]
pub async fn sprite_provision_claude(
    name: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    tracing::info!("sprite_provision_claude called for '{name}'");
    // Read local credentials
    let home = dirs::home_dir()
        .ok_or_else(|| AppError::Internal("Cannot determine home directory".into()))?;
    let creds_path = home.join(".claude").join(".credentials.json");

    let creds_content = std::fs::read_to_string(&creds_path).map_err(|e| {
        AppError::Internal(format!(
            "Cannot read {}: {e}",
            creds_path.display()
        ))
    })?;

    // Base64 encode to avoid shell escaping issues
    let b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        creds_content.as_bytes(),
    );

    // Push to sprite: create dir, decode and write, set permissions
    let client = state.get_sprites_client()?;
    let cmd = format!(
        "mkdir -p ~/.claude && echo '{}' | base64 -d > ~/.claude/.credentials.json && chmod 600 ~/.claude/.credentials.json",
        b64
    );
    client.exec_command(&name, &cmd).await.map_err(|e| {
        AppError::Internal(format!("Failed to provision credentials on '{name}': {e}"))
    })?;

    Ok(())
}

// ==========================================
// Config
// ==========================================

/// Configure the Sprites API client from settings
#[tauri::command]
pub async fn sprite_configure(
    base_url: String,
    token: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    state.set_sprites_client(base_url.clone(), token.clone());

    let client = state.get_sprites_client()?;
    client.test_connection().await
}

/// Test connection to Sprites API
#[tauri::command]
pub async fn sprite_test_connection(state: State<'_, AppState>) -> Result<String, AppError> {
    let client = state.get_sprites_client()?;
    client.test_connection().await
}
