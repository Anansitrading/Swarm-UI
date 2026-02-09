use crate::error::AppError;
use crate::sprites_api;
use crate::state::{AppState, PtyInfo};
use serde::Serialize;
use tauri::{AppHandle, State};

#[derive(Debug, Serialize)]
pub struct SpriteInfo {
    pub name: String,
    pub status: String,
    pub id: Option<String>,
    pub region: Option<String>,
}

/// List all sprites via REST API
#[tauri::command]
pub async fn sprite_list(state: State<'_, AppState>) -> Result<Vec<SpriteInfo>, AppError> {
    let client = state.get_sprites_client()?;
    let sprites = client.list_sprites().await?;

    Ok(sprites
        .into_iter()
        .map(|s| SpriteInfo {
            name: s.name,
            status: s.status,
            id: s.id,
            region: s.region,
        })
        .collect())
}

/// Execute a command on a sprite via REST API (non-interactive)
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

/// Create a checkpoint for a sprite via REST API
#[tauri::command]
pub async fn sprite_checkpoint_create(
    name: String,
    description: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let client = state.get_sprites_client()?;
    let checkpoint = client.create_checkpoint(&name, &description).await?;
    Ok(format!("Checkpoint {} created", checkpoint.id))
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

/// Restore a checkpoint
#[tauri::command]
pub async fn sprite_restore_checkpoint(
    name: String,
    checkpoint_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let client = state.get_sprites_client()?;
    client.restore_checkpoint(&name, &checkpoint_id).await
}

/// Delete a sprite
#[tauri::command]
pub async fn sprite_delete(name: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let client = state.get_sprites_client()?;
    client.delete_sprite(&name).await
}

/// Create a new sprite
#[tauri::command]
pub async fn sprite_create(
    name: String,
    state: State<'_, AppState>,
) -> Result<SpriteInfo, AppError> {
    let client = state.get_sprites_client()?;
    let sprite = client.create_sprite(&name).await?;
    Ok(SpriteInfo {
        name: sprite.name,
        status: sprite.status,
        id: sprite.id,
        region: sprite.region,
    })
}

/// Spawn an interactive WebSocket terminal to a sprite
#[tauri::command]
pub async fn sprite_ws_spawn(
    sprite_name: String,
    cols: Option<u16>,
    rows: Option<u16>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<PtyInfo, AppError> {
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

/// List exec sessions running on a sprite
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
    // Parse process list into session entries
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
        // Format: "1738000000.123 /root/.claude/projects/-home-devuser-MyProject/abc123.jsonl"
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() == 2 {
            let path = parts[1];
            // Extract project dir name and session id from path
            if let Some(fname) = path.rsplit('/').next() {
                let session_id = fname.trim_end_matches(".jsonl").to_string();
                // Extract project path from parent directory name
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

/// Configure the Sprites API client from settings
#[tauri::command]
pub async fn sprite_configure(
    base_url: String,
    token: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    state.set_sprites_client(base_url.clone(), token.clone());

    // Test connection
    let client = state.get_sprites_client()?;
    client.test_connection().await
}

/// Test connection to Sprites API
#[tauri::command]
pub async fn sprite_test_connection(state: State<'_, AppState>) -> Result<String, AppError> {
    let client = state.get_sprites_client()?;
    client.test_connection().await
}
