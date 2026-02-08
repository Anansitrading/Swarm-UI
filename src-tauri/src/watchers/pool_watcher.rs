use crate::error::AppError;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Bot pool slot from ~/.cortex/sprite-pool.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotSlot {
    pub slot: u32,
    pub bot_name: Option<String>,
    pub sprite_name: Option<String>,
    pub status: String,
    pub ticket_id: Option<String>,
    pub role: Option<String>,
    pub claimed_at: Option<String>,
    pub heartbeat: Option<String>,
}

/// Full pool state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolState {
    pub slots: Vec<BotSlot>,
    pub total: usize,
    pub active: usize,
    pub idle: usize,
}

/// Get current bot pool state
#[tauri::command]
pub async fn get_bot_pool_state() -> Result<PoolState, AppError> {
    let pool_path = dirs::home_dir()
        .ok_or_else(|| AppError::Internal("No home dir".into()))?
        .join(".cortex")
        .join("sprite-pool.json");

    if !pool_path.exists() {
        return Ok(PoolState {
            slots: vec![],
            total: 0,
            active: 0,
            idle: 0,
        });
    }

    let content = fs::read_to_string(&pool_path)?;
    let slots: Vec<BotSlot> = serde_json::from_str(&content).unwrap_or_default();

    let total = slots.len();
    let active = slots.iter().filter(|s| s.status == "active" || s.status == "claimed").count();
    let idle = total - active;

    Ok(PoolState {
        slots,
        total,
        active,
        idle,
    })
}

/// Start watching sprite pool file for changes
#[tauri::command]
pub async fn start_pool_watcher(app: AppHandle) -> Result<(), AppError> {
    let pool_path = dirs::home_dir()
        .ok_or_else(|| AppError::Internal("No home dir".into()))?
        .join(".cortex")
        .join("sprite-pool.json");

    let pool_dir = pool_path
        .parent()
        .ok_or_else(|| AppError::Internal("No parent dir".into()))?
        .to_path_buf();

    if !pool_dir.exists() {
        return Err(AppError::NotFound("~/.cortex/ not found".to_string()));
    }

    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel();

        let mut watcher = match RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("Failed to create pool watcher: {e}");
                return;
            }
        };

        if let Err(e) = watcher.watch(&pool_dir, RecursiveMode::NonRecursive) {
            tracing::error!("Failed to watch pool dir: {e}");
            return;
        }

        tracing::info!("Watching sprite pool at {}", pool_path.display());

        loop {
            match rx.recv_timeout(Duration::from_secs(10)) {
                Ok(event) => {
                    for path in &event.paths {
                        if path.file_name().and_then(|n| n.to_str()) == Some("sprite-pool.json") {
                            if let Ok(content) = fs::read_to_string(path) {
                                if let Ok(slots) = serde_json::from_str::<Vec<BotSlot>>(&content) {
                                    let total = slots.len();
                                    let active = slots
                                        .iter()
                                        .filter(|s| {
                                            s.status == "active" || s.status == "claimed"
                                        })
                                        .count();
                                    let state = PoolState {
                                        slots,
                                        total,
                                        active,
                                        idle: total - active,
                                    };
                                    let _ = app.emit("pool:updated", &state);
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
