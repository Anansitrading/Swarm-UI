use crate::error::AppError;
use serde::Serialize;
use std::fs;

#[derive(Debug, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub cmdline: String,
    pub cwd: String,
}

/// Find all running Claude Code processes by scanning /proc
#[tauri::command]
pub async fn find_claude_processes() -> Result<Vec<ProcessInfo>, AppError> {
    let mut processes = Vec::new();

    let proc_dir = fs::read_dir("/proc")?;
    for entry in proc_dir.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Only numeric directories (PIDs)
        let pid: u32 = match name_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let cmdline_path = entry.path().join("cmdline");
        let cwd_path = entry.path().join("cwd");

        let cmdline = match fs::read_to_string(&cmdline_path) {
            Ok(c) => c.replace('\0', " ").trim().to_string(),
            Err(_) => continue,
        };

        // Check if this is a Claude-related process
        if !cmdline.contains("claude") && !cmdline.contains("Claude") {
            continue;
        }

        let cwd = fs::read_link(&cwd_path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        processes.push(ProcessInfo { pid, cmdline, cwd });
    }

    Ok(processes)
}

/// Kill a process by PID (SIGTERM, then SIGKILL after grace period)
#[tauri::command]
pub async fn kill_process(pid: u32, force: Option<bool>) -> Result<(), AppError> {
    let signal = if force.unwrap_or(false) {
        libc::SIGKILL
    } else {
        libc::SIGTERM
    };

    let result = unsafe { libc::kill(pid as i32, signal) };
    if result != 0 {
        return Err(AppError::Internal(format!(
            "Failed to send signal to PID {pid}"
        )));
    }

    Ok(())
}
