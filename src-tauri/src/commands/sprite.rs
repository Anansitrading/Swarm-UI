use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct SpriteInfo {
    pub name: String,
    pub status: String,
    pub id: Option<String>,
}

/// List all sprites via `sprite list` CLI
#[tauri::command]
pub async fn sprite_list() -> Result<Vec<SpriteInfo>, AppError> {
    let output = Command::new("sprite")
        .arg("list")
        .output()
        .map_err(|e| AppError::Internal(format!("Failed to run sprite CLI: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut sprites = Vec::new();

    // Parse sprite list output (tab-separated: name, status, id)
    for line in stdout.lines().skip(1) {
        // Skip header
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            sprites.push(SpriteInfo {
                name: parts[0].to_string(),
                status: parts[1].to_string(),
                id: parts.get(2).map(|s| s.to_string()),
            });
        }
    }

    Ok(sprites)
}

/// Execute a command on a sprite via `sprite exec`
#[tauri::command]
pub async fn sprite_exec(name: String, command: String) -> Result<String, AppError> {
    let output = Command::new("sprite")
        .args(["exec", "-s", &name, &command])
        .output()
        .map_err(|e| AppError::Internal(format!("sprite exec failed: {e}")))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Create a checkpoint for a sprite
#[tauri::command]
pub async fn sprite_checkpoint_create(
    name: String,
    description: String,
) -> Result<String, AppError> {
    let output = Command::new("sprite")
        .args(["checkpoint", "create", "-s", &name, "-c", &description])
        .output()
        .map_err(|e| AppError::Internal(format!("sprite checkpoint failed: {e}")))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
