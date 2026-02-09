use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::fs;

/// A member of an agent team
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMember {
    pub agent_id: String,
    pub name: String,
    pub agent_type: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub joined_at: Option<u64>,
    #[serde(default)]
    pub tmux_pane_id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
}

/// Raw team config as stored in ~/.claude/teams/{name}/config.json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamConfig {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub created_at: Option<u64>,
    #[serde(default)]
    pub lead_agent_id: Option<String>,
    #[serde(default)]
    pub lead_session_id: Option<String>,
    #[serde(default)]
    pub members: Vec<TeamMember>,
}

/// A task in a team's task list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamTask {
    pub id: String,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "activeForm")]
    pub active_form: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub blocks: Vec<String>,
    #[serde(default, rename = "blockedBy")]
    pub blocked_by: Vec<String>,
}

/// Full team info returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamInfo {
    pub name: String,
    pub description: Option<String>,
    pub created_at: Option<u64>,
    pub lead_agent_id: Option<String>,
    pub lead_session_id: Option<String>,
    pub members: Vec<TeamMember>,
    pub tasks: Vec<TeamTask>,
    pub task_summary: TaskSummary,
    pub has_inboxes: bool,
}

/// Summary of task states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
}

/// List all agent teams from ~/.claude/teams/
#[tauri::command]
pub async fn list_teams() -> Result<Vec<TeamInfo>, AppError> {
    let teams_dir = dirs::home_dir()
        .ok_or_else(|| AppError::Internal("No home dir".into()))?
        .join(".claude")
        .join("teams");

    if !teams_dir.exists() {
        return Ok(vec![]);
    }

    let mut teams = Vec::new();

    for entry in fs::read_dir(&teams_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let config_path = path.join("config.json");
        if !config_path.exists() {
            continue;
        }

        let team_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        match read_team(&team_name) {
            Ok(info) => teams.push(info),
            Err(e) => {
                tracing::warn!("Failed to read team {team_name}: {e}");
            }
        }
    }

    // Sort by created_at descending (most recent first)
    teams.sort_by(|a, b| b.created_at.unwrap_or(0).cmp(&a.created_at.unwrap_or(0)));

    Ok(teams)
}

/// Get a single team's full info
#[tauri::command]
pub async fn get_team(name: String) -> Result<TeamInfo, AppError> {
    read_team(&name)
}

/// Read team config + tasks for a given team name
fn read_team(team_name: &str) -> Result<TeamInfo, AppError> {
    let home = dirs::home_dir().ok_or_else(|| AppError::Internal("No home dir".into()))?;

    let config_path = home
        .join(".claude")
        .join("teams")
        .join(team_name)
        .join("config.json");

    if !config_path.exists() {
        return Err(AppError::NotFound(format!("Team {team_name} not found")));
    }

    let content = fs::read_to_string(&config_path)?;
    let config: TeamConfig = serde_json::from_str(&content)?;

    // Read tasks from ~/.claude/tasks/{team_name}/
    let tasks = read_team_tasks(team_name, &home);

    // Check for inboxes directory
    let inboxes_dir = home
        .join(".claude")
        .join("teams")
        .join(team_name)
        .join("inboxes");
    let has_inboxes = inboxes_dir.exists() && inboxes_dir.is_dir();

    let total = tasks.len();
    let pending = tasks
        .iter()
        .filter(|t| t.status.as_deref() == Some("pending"))
        .count();
    let in_progress = tasks
        .iter()
        .filter(|t| {
            t.status.as_deref() == Some("in_progress") || t.status.as_deref() == Some("in-progress")
        })
        .count();
    let completed = tasks
        .iter()
        .filter(|t| t.status.as_deref() == Some("completed"))
        .count();

    Ok(TeamInfo {
        name: config.name,
        description: config.description,
        created_at: config.created_at,
        lead_agent_id: config.lead_agent_id,
        lead_session_id: config.lead_session_id,
        members: config.members,
        tasks,
        task_summary: TaskSummary {
            total,
            pending,
            in_progress,
            completed,
        },
        has_inboxes,
    })
}

/// Read tasks from ~/.claude/tasks/{team_name}/*.json
fn read_team_tasks(team_name: &str, home: &std::path::Path) -> Vec<TeamTask> {
    let tasks_dir = home.join(".claude").join("tasks").join(team_name);

    if !tasks_dir.exists() {
        // Also try matching by UUID-named dirs that might correspond
        // For now just return empty if exact name match doesn't exist
        return vec![];
    }

    let mut tasks = Vec::new();

    if let Ok(entries) = fs::read_dir(&tasks_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            // Only read .json files, skip .lock and .highwatermark
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&path) {
                match serde_json::from_str::<TeamTask>(&content) {
                    Ok(task) => tasks.push(task),
                    Err(e) => {
                        tracing::warn!("Failed to parse task {}: {e}", path.display());
                    }
                }
            }
        }
    }

    // Sort tasks by ID (numeric sort)
    tasks.sort_by(|a, b| {
        let a_num: u32 = a.id.parse().unwrap_or(u32::MAX);
        let b_num: u32 = b.id.parse().unwrap_or(u32::MAX);
        a_num.cmp(&b_num)
    });

    tasks
}
