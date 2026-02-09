use crate::error::AppError;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// REST API client for Sprites.dev
pub struct SpritesClient {
    base_url: String,
    token: String,
    http: Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteInfo {
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteDetail {
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub cpu: Option<String>,
    #[serde(default)]
    pub memory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
    #[serde(default)]
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecSession {
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub command: Option<String>,
}

impl SpritesClient {
    pub fn new(base_url: String, token: String) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            http,
        }
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/v1{}", self.base_url, path)
    }

    /// List all sprites
    pub async fn list_sprites(&self) -> Result<Vec<SpriteInfo>, AppError> {
        let resp = self
            .http
            .get(self.api_url("/sprites"))
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Sprites API request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Sprites API returned {status}: {body}"
            )));
        }

        // The API may return either a list or an object with a list
        let body = resp
            .text()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to read response: {e}")))?;

        // Try parsing as array first
        if let Ok(sprites) = serde_json::from_str::<Vec<SpriteInfo>>(&body) {
            return Ok(sprites);
        }

        // Try as object with "sprites" key
        #[derive(Deserialize)]
        struct SpritesResponse {
            sprites: Vec<SpriteInfo>,
        }
        if let Ok(resp) = serde_json::from_str::<SpritesResponse>(&body) {
            return Ok(resp.sprites);
        }

        // Try as object with "data" key
        #[derive(Deserialize)]
        struct DataResponse {
            data: Vec<SpriteInfo>,
        }
        if let Ok(resp) = serde_json::from_str::<DataResponse>(&body) {
            return Ok(resp.data);
        }

        Err(AppError::Internal(format!(
            "Unexpected sprites response format: {}",
            &body[..body.len().min(200)]
        )))
    }

    /// Get a single sprite
    pub async fn get_sprite(&self, name: &str) -> Result<SpriteDetail, AppError> {
        let resp = self
            .http
            .get(self.api_url(&format!("/sprites/{name}")))
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Sprites API request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Sprites API returned {status}: {body}"
            )));
        }

        resp.json::<SpriteDetail>()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse sprite detail: {e}")))
    }

    /// Create a new sprite
    pub async fn create_sprite(&self, name: &str) -> Result<SpriteInfo, AppError> {
        let resp = self
            .http
            .post(self.api_url("/sprites"))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "name": name }))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Sprites API request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Sprites API returned {status}: {body}"
            )));
        }

        resp.json::<SpriteInfo>()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse sprite info: {e}")))
    }

    /// Delete a sprite
    pub async fn delete_sprite(&self, name: &str) -> Result<(), AppError> {
        let resp = self
            .http
            .delete(self.api_url(&format!("/sprites/{name}")))
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Sprites API request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Sprites API returned {status}: {body}"
            )));
        }

        Ok(())
    }

    /// Execute a command via HTTP (non-interactive)
    pub async fn exec_http(&self, name: &str, cmd: &str) -> Result<ExecResult, AppError> {
        let resp = self
            .http
            .post(self.api_url(&format!("/sprites/{name}/exec")))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "command": cmd }))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Sprites exec request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Sprites exec returned {status}: {body}"
            )));
        }

        resp.json::<ExecResult>()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse exec result: {e}")))
    }

    /// List exec sessions
    pub async fn list_exec_sessions(&self, name: &str) -> Result<Vec<ExecSession>, AppError> {
        let resp = self
            .http
            .get(self.api_url(&format!("/sprites/{name}/exec")))
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Sprites API request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Sprites API returned {status}: {body}"
            )));
        }

        resp.json::<Vec<ExecSession>>()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse exec sessions: {e}")))
    }

    /// List checkpoints for a sprite
    pub async fn list_checkpoints(&self, name: &str) -> Result<Vec<Checkpoint>, AppError> {
        let resp = self
            .http
            .get(self.api_url(&format!("/sprites/{name}/checkpoints")))
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Sprites API request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Sprites API returned {status}: {body}"
            )));
        }

        resp.json::<Vec<Checkpoint>>()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse checkpoints: {e}")))
    }

    /// Create a checkpoint
    pub async fn create_checkpoint(
        &self,
        name: &str,
        comment: &str,
    ) -> Result<Checkpoint, AppError> {
        let resp = self
            .http
            .post(self.api_url(&format!("/sprites/{name}/checkpoint")))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "comment": comment }))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Sprites API request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Sprites API returned {status}: {body}"
            )));
        }

        resp.json::<Checkpoint>()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse checkpoint: {e}")))
    }

    /// Restore a checkpoint
    pub async fn restore_checkpoint(
        &self,
        name: &str,
        checkpoint_id: &str,
    ) -> Result<(), AppError> {
        let resp = self
            .http
            .post(self.api_url(&format!(
                "/sprites/{name}/checkpoints/{checkpoint_id}/restore"
            )))
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Sprites API request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Sprites API returned {status}: {body}"
            )));
        }

        Ok(())
    }

    /// Get the WebSocket URL for exec with TTY
    pub fn ws_exec_url(&self, name: &str, cols: u16, rows: u16) -> String {
        let base = self
            .base_url
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        format!(
            "{}/v1/sprites/{}/exec?tty=true&cmd=/bin/bash&cols={}&rows={}",
            base, name, cols, rows
        )
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the bearer token (needed for WebSocket auth header)
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Test connection by listing sprites
    pub async fn test_connection(&self) -> Result<String, AppError> {
        let sprites = self.list_sprites().await?;
        Ok(format!("Connected. Found {} sprites.", sprites.len()))
    }
}
