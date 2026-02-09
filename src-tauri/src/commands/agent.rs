use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::fs;

/// Agent definition discovered from ~/.claude/agents/
#[derive(Debug, Serialize, Clone)]
pub struct AgentDef {
    pub name: String,
    pub file_path: String,
    pub description: Option<String>,
}

/// Smith override configuration for a session
#[derive(Debug, Serialize, Deserialize)]
pub struct SmithOverride {
    pub enabled: bool,
    pub instructions: String,
}

/// Discover available agents from ~/.claude/agents/ directory
/// Returns built-in agents (claude) plus any .md agent definitions found
#[tauri::command]
pub async fn list_agents() -> Result<Vec<AgentDef>, AppError> {
    let mut agents = vec![AgentDef {
        name: "claude".to_string(),
        file_path: String::new(),
        description: Some("Default Claude Code session".to_string()),
    }];

    let home = dirs::home_dir().ok_or_else(|| AppError::Internal("No home dir".into()))?;
    let agents_dir = home.join(".claude").join("agents");

    if agents_dir.exists() {
        for entry in fs::read_dir(&agents_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }

            // Read first line for description
            let content = fs::read_to_string(&path).unwrap_or_default();
            let description = content
                .lines()
                .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
                .or_else(|| content.lines().find(|l| l.starts_with('#')))
                .map(|l| l.trim_start_matches('#').trim().to_string());

            agents.push(AgentDef {
                name,
                file_path: path.to_string_lossy().to_string(),
                description,
            });
        }
    }

    // Sort: claude first, then alphabetical
    agents[1..].sort_by(|a, b| a.name.cmp(&b.name));

    Ok(agents)
}

/// Discover agents available on a specific sprite
#[tauri::command]
pub async fn list_sprite_agents(
    name: String,
    state: tauri::State<'_, crate::state::AppState>,
) -> Result<Vec<AgentDef>, AppError> {
    let mut agents = vec![AgentDef {
        name: "claude".to_string(),
        file_path: String::new(),
        description: Some("Default Claude Code session".to_string()),
    }];

    let client = state.get_sprites_client()?;
    let result = client
        .exec_http(
            &name,
            "ls ~/.claude/agents/*.md 2>/dev/null | while read f; do echo \"$(basename \"$f\" .md)|$(head -5 \"$f\" | grep -v '^$' | head -1)\"; done",
        )
        .await?;

    for line in result.stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(2, '|').collect();
        let agent_name = parts[0].to_string();
        let desc = parts
            .get(1)
            .map(|s| s.trim_start_matches('#').trim().to_string());
        agents.push(AgentDef {
            name: agent_name,
            file_path: String::new(),
            description: desc,
        });
    }

    Ok(agents)
}

/// Save Smith override configuration for a session
#[tauri::command]
pub async fn save_smith_override(
    session_id: String,
    enabled: bool,
    instructions: String,
) -> Result<(), AppError> {
    let home = dirs::home_dir().ok_or_else(|| AppError::Internal("No home dir".into()))?;
    let dir = home.join(".claude").join("smith-overrides");
    fs::create_dir_all(&dir)?;

    let path = dir.join(format!("{}.json", session_id));
    let data = SmithOverride {
        enabled,
        instructions,
    };
    let json =
        serde_json::to_string_pretty(&data).map_err(|e| AppError::Internal(e.to_string()))?;
    fs::write(&path, json)?;

    Ok(())
}

/// Load Smith override configuration for a session
#[tauri::command]
pub async fn load_smith_override(session_id: String) -> Result<SmithOverride, AppError> {
    let home = dirs::home_dir().ok_or_else(|| AppError::Internal("No home dir".into()))?;
    let path = home
        .join(".claude")
        .join("smith-overrides")
        .join(format!("{}.json", session_id));

    if !path.exists() {
        return Ok(SmithOverride {
            enabled: false,
            instructions: String::new(),
        });
    }

    let content = fs::read_to_string(&path)?;
    let data: SmithOverride =
        serde_json::from_str(&content).map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(data)
}
