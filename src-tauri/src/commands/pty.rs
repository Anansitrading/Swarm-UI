use crate::error::AppError;
use crate::state::{AppState, PtyInfo, PtyInstance, PtySpawnConfig};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::Read;
use std::thread;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

/// Spawn a new PTY process and return its info.
/// The PTY output is streamed via events: "pty:data:{id}"
#[tauri::command]
pub async fn pty_spawn(
    config: PtySpawnConfig,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<PtyInfo, AppError> {
    let id = Uuid::new_v4().to_string();
    let cols = config.cols.unwrap_or(80);
    let rows = config.rows.unwrap_or(24);

    let pty_system = NativePtySystem::default();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| AppError::Pty(e.to_string()))?;

    let shell = config
        .shell
        .unwrap_or_else(|| std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string()));

    let mut cmd = CommandBuilder::new(&shell);
    if let Some(args) = &config.args {
        for arg in args {
            cmd.arg(arg);
        }
    }
    if let Some(cwd) = &config.cwd {
        cmd.cwd(cwd);
    }
    if let Some(env) = &config.env {
        for (k, v) in env {
            cmd.env(k, v);
        }
    }
    // Ensure TERM is set for proper terminal emulation
    cmd.env("TERM", "xterm-256color");

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| AppError::Pty(e.to_string()))?;

    let pid = child.process_id().unwrap_or(0);

    // Get writer for sending input to the PTY
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| AppError::Pty(e.to_string()))?;

    // Get reader for reading output from the PTY
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| AppError::Pty(e.to_string()))?;

    let pty_id = id.clone();
    let info = PtyInfo {
        id: id.clone(),
        pid,
        cols,
        rows,
    };

    // Store the PTY instance
    {
        let mut ptys = state.ptys.lock().unwrap();
        ptys.insert(
            id.clone(),
            PtyInstance {
                id: id.clone(),
                master: pair.master,
                writer,
                child,
                cols,
                rows,
            },
        );
    }

    // Spawn a thread to read PTY output and emit events
    let event_name = format!("pty:data:{}", pty_id);
    let exit_event = format!("pty:exit:{}", pty_id);
    thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    // Emit raw bytes as base64 to preserve binary data
                    let data = base64_encode(&buf[..n]);
                    let _ = app.emit(&event_name, data);
                }
                Err(_) => break,
            }
        }
        let _ = app.emit(&exit_event, ());
    });

    Ok(info)
}

/// Write data to a PTY (user keystrokes from xterm.js)
#[tauri::command]
pub async fn pty_write(
    id: String,
    data: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut ptys = state.ptys.lock().unwrap();
    let pty = ptys
        .get_mut(&id)
        .ok_or_else(|| AppError::NotFound(format!("PTY {id} not found")))?;

    // Data comes as base64 from frontend
    let bytes = base64_decode(&data)?;
    use std::io::Write;
    pty.writer
        .write_all(&bytes)
        .map_err(|e| AppError::Pty(e.to_string()))?;
    pty.writer
        .flush()
        .map_err(|e| AppError::Pty(e.to_string()))?;

    Ok(())
}

/// Resize a PTY
#[tauri::command]
pub async fn pty_resize(
    id: String,
    cols: u16,
    rows: u16,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut ptys = state.ptys.lock().unwrap();
    let pty = ptys
        .get_mut(&id)
        .ok_or_else(|| AppError::NotFound(format!("PTY {id} not found")))?;

    pty.master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| AppError::Pty(e.to_string()))?;

    pty.cols = cols;
    pty.rows = rows;

    Ok(())
}

/// Kill a PTY process and clean up
#[tauri::command]
pub async fn pty_kill(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let mut ptys = state.ptys.lock().unwrap();
    if let Some(mut pty) = ptys.remove(&id) {
        let _ = pty.child.kill();
    }
    Ok(())
}

/// List all active PTY instances
#[tauri::command]
pub async fn pty_list(state: State<'_, AppState>) -> Result<Vec<PtyInfo>, AppError> {
    let ptys = state.ptys.lock().unwrap();
    let infos: Vec<PtyInfo> = ptys
        .values()
        .map(|p| PtyInfo {
            id: p.id.clone(),
            pid: p.child.process_id().unwrap_or(0),
            cols: p.cols,
            rows: p.rows,
        })
        .collect();
    Ok(infos)
}

/// Public base64 encode for use by other command modules
pub fn base64_encode_pub(data: &[u8]) -> String {
    base64_encode(data)
}

// Simple base64 encode/decode to avoid adding another dep
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(data: &str) -> Result<Vec<u8>, AppError> {
    fn val(c: u8) -> Result<u32, AppError> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            b'=' => Ok(0),
            _ => Err(AppError::Internal(format!("Invalid base64 char: {c}"))),
        }
    }

    let bytes = data.as_bytes();
    let mut result = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        if chunk.len() < 4 {
            break;
        }
        let a = val(chunk[0])?;
        let b = val(chunk[1])?;
        let c = val(chunk[2])?;
        let d = val(chunk[3])?;
        let triple = (a << 18) | (b << 12) | (c << 6) | d;
        result.push(((triple >> 16) & 0xFF) as u8);
        if chunk[2] != b'=' {
            result.push(((triple >> 8) & 0xFF) as u8);
        }
        if chunk[3] != b'=' {
            result.push((triple & 0xFF) as u8);
        }
    }
    Ok(result)
}
