use crate::error::AppError;
use serde::Serialize;
use sysinfo::System;

#[derive(Debug, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub cmdline: String,
    pub cwd: String,
}

/// Find all running Claude Code processes (cross-platform via sysinfo)
#[tauri::command]
pub async fn find_claude_processes() -> Result<Vec<ProcessInfo>, AppError> {
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let mut processes = Vec::new();

    for (pid, process) in sys.processes() {
        let cmd_parts: Vec<&str> = process
            .cmd()
            .iter()
            .map(|s| s.to_str().unwrap_or(""))
            .collect();
        let cmdline = cmd_parts.join(" ");

        if !cmdline.contains("claude") && !cmdline.contains("Claude") {
            continue;
        }

        let cwd = process
            .cwd()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        processes.push(ProcessInfo {
            pid: pid.as_u32(),
            cmdline,
            cwd,
        });
    }

    Ok(processes)
}

/// Kill a process by PID (cross-platform)
#[tauri::command]
pub async fn kill_process(pid: u32, force: Option<bool>) -> Result<(), AppError> {
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let sysinfo_pid = sysinfo::Pid::from_u32(pid);
    let process = sys
        .process(sysinfo_pid)
        .ok_or_else(|| AppError::NotFound(format!("Process {pid} not found")))?;

    if force.unwrap_or(false) {
        process.kill();
    } else {
        // On Unix this sends SIGTERM, on Windows it terminates the process
        #[cfg(unix)]
        {
            let result = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
            if result != 0 {
                return Err(AppError::Internal(format!(
                    "Failed to send SIGTERM to PID {pid}"
                )));
            }
        }
        #[cfg(windows)]
        {
            process.kill();
        }
    }

    Ok(())
}
