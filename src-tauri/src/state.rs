use portable_pty::MasterPty;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Mutex;

use crate::sprites_api::SpritesClient;
use crate::sprites_ws::WsState;

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
    pub sprites_client: Mutex<Option<SpritesClient>>,
    pub ws_state: WsState,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            ptys: Mutex::new(HashMap::new()),
            sprites_client: Mutex::new(None),
            ws_state: WsState::new(),
        }
    }

    /// Update the sprites client when settings change
    pub fn set_sprites_client(&self, base_url: String, token: String) {
        let client = SpritesClient::new(base_url, token);
        *self.sprites_client.lock().unwrap() = Some(client);
    }

    /// Get a reference to the sprites client, returning error if not configured
    pub fn get_sprites_client(&self) -> Result<SpritesClient, crate::error::AppError> {
        let guard = self.sprites_client.lock().unwrap();
        match &*guard {
            Some(client) => {
                // Clone the client data to create a new one (SpritesClient is cheap to recreate)
                Ok(SpritesClient::new(
                    client.base_url().to_string(),
                    client.token().to_string(),
                ))
            }
            None => Err(crate::error::AppError::Internal(
                "Sprites API not configured. Go to Settings to enter your API token.".to_string(),
            )),
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
