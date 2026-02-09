use crate::commands::team::{TaskSummary, TeamConfig, TeamInfo, TeamTask};
use crate::error::AppError;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Start watching ~/.claude/teams/ and ~/.claude/tasks/ for changes.
/// Emits "team:updated" events when team configs or tasks change.
#[tauri::command]
pub async fn start_team_watcher(app: AppHandle) -> Result<(), AppError> {
    let home = dirs::home_dir().ok_or_else(|| AppError::Internal("No home dir".into()))?;

    let teams_dir = home.join(".claude").join("teams");
    let tasks_dir = home.join(".claude").join("tasks");

    // Create dirs if they don't exist (they might not exist yet)
    if !teams_dir.exists() {
        tracing::info!("~/.claude/teams/ not found, team watcher will wait");
    }

    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel();

        let mut watcher = match RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(3)),
        ) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("Failed to create team watcher: {e}");
                return;
            }
        };

        // Watch teams dir if it exists
        if teams_dir.exists() {
            if let Err(e) = watcher.watch(&teams_dir, RecursiveMode::Recursive) {
                tracing::error!("Failed to watch teams dir: {e}");
            } else {
                tracing::info!("Watching {} for team changes", teams_dir.display());
            }
        }

        // Watch tasks dir if it exists
        if tasks_dir.exists() {
            if let Err(e) = watcher.watch(&tasks_dir, RecursiveMode::Recursive) {
                tracing::error!("Failed to watch tasks dir: {e}");
            } else {
                tracing::info!("Watching {} for task changes", tasks_dir.display());
            }
        }

        loop {
            match rx.recv_timeout(Duration::from_secs(5)) {
                Ok(event) => {
                    for path in &event.paths {
                        let path_str = path.to_string_lossy();

                        // Only react to .json file changes
                        if path.extension().and_then(|e| e.to_str()) != Some("json") {
                            continue;
                        }

                        // Determine which team was affected
                        let team_name = extract_team_name(&path_str);
                        if let Some(name) = team_name {
                            // Re-read the full team info and emit.
                            // Silently skip teams whose config was deleted.
                            match read_team_for_event(&name) {
                                Ok(info) => {
                                    let _ = app.emit("team:updated", &info);
                                }
                                Err(AppError::NotFound(_)) => {}
                                Err(e) => {
                                    tracing::warn!("Failed to read team {name} for event: {e}");
                                }
                            }
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Ok(())
}

/// Extract team name from a path like ~/.claude/teams/{name}/config.json
/// or ~/.claude/tasks/{name}/1.json
fn extract_team_name(path: &str) -> Option<String> {
    // Look for /teams/{name}/ or /tasks/{name}/
    if let Some(idx) = path.find("/.claude/teams/") {
        let after = &path[idx + "/.claude/teams/".len()..];
        return after.split('/').next().map(|s| s.to_string());
    }
    if let Some(idx) = path.find("/.claude/tasks/") {
        let after = &path[idx + "/.claude/tasks/".len()..];
        return after.split('/').next().map(|s| s.to_string());
    }
    None
}

/// Read team info for emitting events (same logic as commands::team but standalone)
fn read_team_for_event(team_name: &str) -> Result<TeamInfo, AppError> {
    let home = dirs::home_dir().ok_or_else(|| AppError::Internal("No home dir".into()))?;

    let config_path = home
        .join(".claude")
        .join("teams")
        .join(team_name)
        .join("config.json");

    if !config_path.exists() {
        return Err(AppError::NotFound(format!(
            "Team config not found: {team_name}"
        )));
    }

    let content = fs::read_to_string(&config_path)?;
    let config: TeamConfig = serde_json::from_str(&content)?;

    // Read tasks
    let tasks_dir = home.join(".claude").join("tasks").join(team_name);
    let mut tasks = Vec::new();

    if tasks_dir.exists() {
        if let Ok(entries) = fs::read_dir(&tasks_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(task) = serde_json::from_str::<TeamTask>(&content) {
                        tasks.push(task);
                    }
                }
            }
        }
    }

    tasks.sort_by(|a, b| {
        let a_num: u32 = a.id.parse().unwrap_or(u32::MAX);
        let b_num: u32 = b.id.parse().unwrap_or(u32::MAX);
        a_num.cmp(&b_num)
    });

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

    let inboxes_dir = home
        .join(".claude")
        .join("teams")
        .join(team_name)
        .join("inboxes");

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
        has_inboxes: inboxes_dir.exists(),
    })
}
