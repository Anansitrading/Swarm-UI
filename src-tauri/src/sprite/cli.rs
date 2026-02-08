/// Sprite CLI subprocess wrapper.
/// Sprite commands are executed via the `sprite` binary which handles
/// WebSocket connections and authentication internally.
///
/// For terminal attach (sprite console), we spawn the CLI as a PTY child
/// so that xterm.js can interact with it directly.
///
/// See also: commands/sprite.rs for Tauri command wrappers
/// See also: commands/pty.rs for PTY-based console attach

use std::process::Command;

/// Check if sprite CLI is available
pub fn is_available() -> bool {
    Command::new("sprite")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get sprite CLI version
pub fn version() -> Option<String> {
    Command::new("sprite")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}
