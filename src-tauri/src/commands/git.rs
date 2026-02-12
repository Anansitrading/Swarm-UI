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

/// Get git diff (staged + unstaged) for a repository path
#[tauri::command]
pub async fn get_git_diff(repo_path: String) -> Result<Vec<GitFileChange>, AppError> {
    let mut changes = Vec::new();

    // Get unstaged changes
    let output = Command::new("git")
        .args(["diff", "--name-status"])
        .current_dir(&repo_path)
        .output()
        .map_err(|e| AppError::Internal(format!("git diff failed: {e}")))?;

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let parts: Vec<&str> = line.splitn(2, '\t').collect();
        if parts.len() == 2 {
            changes.push(GitFileChange {
                path: parts[1].to_string(),
                status: parse_git_status(parts[0]),
                staged: false,
            });
        }
    }

    // Get staged changes
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-status"])
        .current_dir(&repo_path)
        .output()
        .map_err(|e| AppError::Internal(format!("git diff --cached failed: {e}")))?;

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let parts: Vec<&str> = line.splitn(2, '\t').collect();
        if parts.len() == 2 {
            changes.push(GitFileChange {
                path: parts[1].to_string(),
                status: parse_git_status(parts[0]),
                staged: true,
            });
        }
    }

    // Get untracked files
    let output = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(&repo_path)
        .output()
        .map_err(|e| AppError::Internal(format!("git ls-files failed: {e}")))?;

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let path = line.trim();
        if !path.is_empty() {
            changes.push(GitFileChange {
                path: path.to_string(),
                status: "untracked".to_string(),
                staged: false,
            });
        }
    }

    Ok(changes)
}

/// Get recent git commits for a repository
#[tauri::command]
pub async fn get_git_log(
    repo_path: String,
    count: Option<u32>,
) -> Result<Vec<GitCommit>, AppError> {
    let n = count.unwrap_or(10);
    let output = Command::new("git")
        .args([
            "log",
            &format!("--max-count={}", n),
            "--format=%H%n%h%n%an%n%ar%n%s%n---END---",
        ])
        .current_dir(&repo_path)
        .output()
        .map_err(|e| AppError::Internal(format!("git log failed: {e}")))?;

    if !output.status.success() {
        // No commits yet or not a git repo
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();
    let mut lines = stdout.lines().peekable();

    while lines.peek().is_some() {
        let hash = match lines.next() {
            Some(h) if !h.is_empty() => h.to_string(),
            _ => break,
        };
        let short_hash = lines.next().unwrap_or("").to_string();
        let author = lines.next().unwrap_or("").to_string();
        let time_ago = lines.next().unwrap_or("").to_string();
        let subject = lines.next().unwrap_or("").to_string();
        // consume ---END--- separator
        let _ = lines.next();

        commits.push(GitCommit {
            hash,
            short_hash,
            author,
            time_ago,
            subject,
        });
    }

    Ok(commits)
}

#[derive(Debug, Serialize)]
pub struct GitCommit {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub time_ago: String,
    pub subject: String,
}

/// Get the content diff for a specific file
#[tauri::command]
pub async fn get_file_diff(
    repo_path: String,
    file_path: String,
    staged: bool,
) -> Result<FileDiffContent, AppError> {
    let full_path = std::path::Path::new(&repo_path).join(&file_path);

    // Check if the file is tracked by git
    let is_tracked = Command::new("git")
        .args(["ls-files", "--error-unmatch", &file_path])
        .current_dir(&repo_path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    // Check if file exists in HEAD (for newly added but staged files)
    let in_head = Command::new("git")
        .args(["cat-file", "-t", &format!("HEAD:{}", file_path)])
        .current_dir(&repo_path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !is_tracked && !in_head {
        // Untracked file: show full content as "new file"
        let content = std::fs::read_to_string(&full_path).unwrap_or_default();
        return Ok(FileDiffContent {
            diff: format!("New untracked file: {}", file_path),
            old_content: String::new(),
            new_content: content,
        });
    }

    let args = if staged {
        vec!["diff", "--cached", "--", &file_path]
    } else {
        vec!["diff", "--", &file_path]
    };

    let output = Command::new("git")
        .args(&args)
        .current_dir(&repo_path)
        .output()
        .map_err(|e| AppError::Internal(format!("git diff file failed: {e}")))?;

    let diff = String::from_utf8_lossy(&output.stdout).to_string();

    // Get old content (HEAD version)
    let old_content = Command::new("git")
        .args(["show", &format!("HEAD:{}", file_path)])
        .current_dir(&repo_path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    // Get new content (working tree or index)
    let new_content = if staged {
        Command::new("git")
            .args(["show", &format!(":{}", file_path)])
            .current_dir(&repo_path)
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default()
    } else {
        std::fs::read_to_string(&full_path).unwrap_or_default()
    };

    Ok(FileDiffContent {
        diff,
        old_content,
        new_content,
    })
}

#[derive(Debug, Serialize)]
pub struct GitFileChange {
    pub path: String,
    pub status: String,
    pub staged: bool,
}

#[derive(Debug, Serialize)]
pub struct FileDiffContent {
    pub diff: String,
    pub old_content: String,
    pub new_content: String,
}

/// Get files changed in a specific commit
#[tauri::command]
pub async fn get_commit_files(
    repo_path: String,
    commit_hash: String,
) -> Result<Vec<GitFileChange>, AppError> {
    let output = Command::new("git")
        .args(["diff-tree", "--no-commit-id", "-r", "--name-status", &commit_hash])
        .current_dir(&repo_path)
        .output()
        .map_err(|e| AppError::Internal(format!("git diff-tree failed: {e}")))?;

    let mut changes = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let parts: Vec<&str> = line.splitn(2, '\t').collect();
        if parts.len() == 2 {
            changes.push(GitFileChange {
                path: parts[1].to_string(),
                status: parse_git_status(parts[0]),
                staged: false,
            });
        }
    }
    Ok(changes)
}

/// Get the diff content for a file in a specific commit (parent..commit)
#[tauri::command]
pub async fn get_commit_file_diff(
    repo_path: String,
    commit_hash: String,
    file_path: String,
) -> Result<FileDiffContent, AppError> {
    // Get old content (parent commit)
    let old_content = Command::new("git")
        .args(["show", &format!("{}~1:{}", commit_hash, file_path)])
        .current_dir(&repo_path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    // Get new content (the commit)
    let new_content = Command::new("git")
        .args(["show", &format!("{}:{}", commit_hash, file_path)])
        .current_dir(&repo_path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    // Get the actual diff
    let diff = Command::new("git")
        .args(["diff", &format!("{}~1", commit_hash), &commit_hash, "--", &file_path])
        .current_dir(&repo_path)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    Ok(FileDiffContent {
        diff,
        old_content,
        new_content,
    })
}

fn parse_git_status(s: &str) -> String {
    match s.chars().next() {
        Some('M') => "modified".to_string(),
        Some('A') => "added".to_string(),
        Some('D') => "deleted".to_string(),
        Some('R') => "renamed".to_string(),
        Some('C') => "copied".to_string(),
        _ => s.to_string(),
    }
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
