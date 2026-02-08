use crate::error::AppError;
use serde::Serialize;
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct WorktreeInfo {
    pub path: String,
    pub branch: String,
    pub is_bare: bool,
}

/// Detect git worktrees for a given repository path
#[tauri::command]
pub async fn detect_worktree(repo_path: String) -> Result<Vec<WorktreeInfo>, AppError> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(&repo_path)
        .output()
        .map_err(|e| AppError::Internal(format!("git worktree list failed: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    let mut current_path = String::new();
    let mut current_branch = String::new();
    let mut is_bare = false;

    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            if !current_path.is_empty() {
                worktrees.push(WorktreeInfo {
                    path: current_path.clone(),
                    branch: current_branch.clone(),
                    is_bare,
                });
            }
            current_path = path.to_string();
            current_branch.clear();
            is_bare = false;
        } else if let Some(branch) = line.strip_prefix("branch refs/heads/") {
            current_branch = branch.to_string();
        } else if line == "bare" {
            is_bare = true;
        }
    }

    if !current_path.is_empty() {
        worktrees.push(WorktreeInfo {
            path: current_path,
            branch: current_branch,
            is_bare,
        });
    }

    Ok(worktrees)
}

/// Get the current git branch for a directory
#[tauri::command]
pub async fn get_git_branch(path: String) -> Result<Option<String>, AppError> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&path)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let branch = String::from_utf8_lossy(&o.stdout).trim().to_string();
            Ok(Some(branch))
        }
        _ => Ok(None),
    }
}
