use crate::error::AppError;
use std::fs;

/// Read a file's contents (for diff viewer)
#[tauri::command]
pub async fn read_file(path: String) -> Result<String, AppError> {
    fs::read_to_string(&path).map_err(|e| AppError::Io(e))
}

/// Read a range of lines from a file
#[tauri::command]
pub async fn read_file_range(
    path: String,
    start_line: usize,
    end_line: usize,
) -> Result<String, AppError> {
    let content = fs::read_to_string(&path)?;
    let lines: Vec<&str> = content.lines().collect();
    let start = start_line.saturating_sub(1).min(lines.len());
    let end = end_line.min(lines.len());
    Ok(lines[start..end].join("\n"))
}
