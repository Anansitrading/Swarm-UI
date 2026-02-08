use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Represents a single PTY instance
pub struct PtyInstance {
    pub id: String,
    pub master: Box<dyn MasterPty + Send>,
    pub writer: Box<dyn Write + Send>,
    pub child: Box<dyn portable_pty::Child + Send>,
    pub cols: u16,
    pub rows: u16,
}

/// Shared application state wrapped in Mutex for thread safety
pub struct AppState {
    pub ptys: Mutex<HashMap<String, PtyInstance>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            ptys: Mutex::new(HashMap::new()),
        }
    }
}

/// PTY spawn configuration from frontend
#[derive(Debug, Deserialize)]
pub struct PtySpawnConfig {
    pub shell: Option<String>,
    pub args: Option<Vec<String>>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
}

/// PTY info returned to frontend
#[derive(Debug, Serialize, Clone)]
pub struct PtyInfo {
    pub id: String,
    pub pid: u32,
    pub cols: u16,
    pub rows: u16,
}
