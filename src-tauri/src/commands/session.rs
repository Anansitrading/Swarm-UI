use crate::error::AppError;
use crate::state::{AppState, PtyInfo, PtyInstance};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::Read;
use std::thread;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

/// Inject a steering message into a Claude Code session by resuming it in a PTY.
/// For idle/waiting sessions, spawns `claude --resume <id>` and sends the message.
/// Returns PtyInfo so the frontend can track the resumed session output.
#[tauri::command]
pub async fn inject_session_message(
    session_id: String,
    message: String,
    cwd: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<PtyInfo, AppError> {
    let id = Uuid::new_v4().to_string();
    let cols: u16 = 120;
    let rows: u16 = 40;

    let pty_system = NativePtySystem::default();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| AppError::Pty(e.to_string()))?;

    // Build the claude --resume command
    let mut cmd = CommandBuilder::new("claude");
    cmd.arg("--resume");
    cmd.arg(&session_id);
    cmd.arg("--dangerously-skip-permissions");
    cmd.cwd(&cwd);
    cmd.env("TERM", "xterm-256color");

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| AppError::Pty(e.to_string()))?;

    let pid = child.process_id().unwrap_or(0);

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| AppError::Pty(e.to_string()))?;

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| AppError::Pty(e.to_string()))?;

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

    // Clone app handle before moving into threads
    let app_for_inject = app.clone();

    // Spawn reader thread to stream PTY output as events
    let pty_id = id.clone();
    let event_name = format!("pty:data:{}", pty_id);
    let exit_event = format!("pty:exit:{}", pty_id);
    thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = crate::commands::pty::base64_encode_pub(&buf[..n]);
                    let _ = app.emit(&event_name, data);
                }
                Err(_) => break,
            }
        }
        let _ = app.emit(&exit_event, ());
    });

    // After a delay for Claude to load, emit an event telling the frontend
    // to send the steering message via pty_write. This avoids ownership issues
    // with the writer (which is stored in PtyInstance).
    let steering_event = format!("pty:inject:{}", id);
    thread::spawn(move || {
        thread::sleep(std::time::Duration::from_millis(3000));
        let _ = app_for_inject.emit(&steering_event, message);
    });

    Ok(info)
}
