use crate::error::AppError;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::ipc::Channel;

const NDJSON_MAX_LINE: usize = 64 * 1024; // 64 KB per event line
const STREAM_TIMEOUT: Duration = Duration::from_secs(300); // checkpoints can take minutes
const LIST_TIMEOUT: Duration = Duration::from_secs(8); // fast REST calls — fail fast

// ── Core sprite types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteInfo {
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteDetail {
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub organization: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub url_settings: Option<UrlSettings>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub last_started_at: Option<String>,
    #[serde(default)]
    pub last_active_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlSettings {
    pub auth: String, // "sprite" | "public"
}

// ── Checkpoint types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub create_time: Option<String>,
    #[serde(default)]
    pub source_id: Option<String>,
}

// ── Exec session types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecSession {
    pub id: serde_json::Value, // API returns string OR number
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub is_active: Option<bool>,
    #[serde(default)]
    pub tty: Option<bool>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub last_activity: Option<String>,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub bytes_per_second: Option<f64>,
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

// ── Service types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceState {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub name: String,
    #[serde(default)]
    pub cmd: String,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub needs: Option<Vec<String>>,
    #[serde(default)]
    pub http_port: Option<u16>,
    #[serde(default)]
    pub state: Option<ServiceState>,
}

// ── NDJSON stream event types (tagged enums) ───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    Info {
        data: Option<String>,
        time: Option<String>,
    },
    Error {
        error: Option<String>,
        data: Option<String>,
        time: Option<String>,
    },
    Complete {
        data: Option<String>,
        time: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServiceStreamEvent {
    Started {
        timestamp: Option<i64>,
    },
    Stopping {
        timestamp: Option<i64>,
    },
    Stopped {
        exit_code: Option<i32>,
        timestamp: Option<i64>,
    },
    Stdout {
        data: Option<String>,
        timestamp: Option<i64>,
    },
    Stderr {
        data: Option<String>,
        timestamp: Option<i64>,
    },
    Error {
        data: Option<String>,
        timestamp: Option<i64>,
    },
    Exit {
        exit_code: Option<i32>,
        timestamp: Option<i64>,
    },
    Complete {
        timestamp: Option<i64>,
        log_files: Option<serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecKillEvent {
    Signal {
        signal: Option<String>,
        pid: Option<u32>,
        message: Option<String>,
    },
    Timeout {
        message: Option<String>,
    },
    Exited {
        message: Option<String>,
    },
    Killed {
        message: Option<String>,
    },
    Error {
        message: Option<String>,
    },
    Complete {
        exit_code: Option<i32>,
    },
}

// ── Terminal predicates ────────────────────────────────────────────────────

impl StreamEvent {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            StreamEvent::Complete { .. } | StreamEvent::Error { .. }
        )
    }
}

impl ServiceStreamEvent {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ServiceStreamEvent::Complete { .. }
                | ServiceStreamEvent::Stopped { .. }
                | ServiceStreamEvent::Exit { .. }
                | ServiceStreamEvent::Error { .. }
        )
    }
}

impl ExecKillEvent {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ExecKillEvent::Complete { .. }
                | ExecKillEvent::Exited { .. }
                | ExecKillEvent::Killed { .. }
                | ExecKillEvent::Error { .. }
        )
    }
}

// ── Generic NDJSON streaming helper ────────────────────────────────────────

pub async fn pipe_ndjson_stream<T>(
    response: reqwest::Response,
    on_event: &Channel<T>,
    is_terminal: impl Fn(&T) -> bool,
) -> Result<(), AppError>
where
    T: for<'de> serde::Deserialize<'de> + serde::Serialize + Clone + Send + 'static,
{
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut received_terminal = false;

    loop {
        match stream.next().await {
            Some(Ok(chunk)) => {
                buffer.push_str(&String::from_utf8_lossy(&chunk));
            }
            Some(Err(e)) => {
                // Flush buffer before deciding whether this error matters
                let flushed = flush_buffer::<T>(&mut buffer, on_event, &is_terminal)?;
                if flushed || received_terminal {
                    // Connection closed after terminal event — this is normal
                    return Ok(());
                }
                return Err(AppError::Internal(format!("ndjson stream read error: {e}")));
            }
            None => {
                // Clean EOF — flush remaining buffer
                flush_buffer::<T>(&mut buffer, on_event, &is_terminal)?;
                return Ok(());
            }
        }

        // Process all complete lines in buffer
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();

            if line.is_empty() {
                continue;
            }
            if line.len() > NDJSON_MAX_LINE {
                return Err(AppError::Internal(format!(
                    "ndjson line too large: {} bytes",
                    line.len()
                )));
            }

            let event = serde_json::from_str::<T>(&line).map_err(|e| {
                AppError::Internal(format!(
                    "ndjson parse error: {e} — line: {}",
                    &line[..line.len().min(120)]
                ))
            })?;

            let terminal = is_terminal(&event);
            on_event
                .send(event)
                .map_err(|e| AppError::Internal(format!("channel send error: {e}")))?;

            if terminal {
                received_terminal = true;
                return Ok(()); // Don't wait for connection close
            }
        }

        if buffer.len() > NDJSON_MAX_LINE {
            return Err(AppError::Internal(format!(
                "ndjson buffer overflow: {} bytes",
                buffer.len()
            )));
        }
    }
}

/// Flush any partial line remaining in buffer after stream ends/errors.
/// Returns true if a terminal event was found and sent.
fn flush_buffer<T>(
    buffer: &mut String,
    on_event: &Channel<T>,
    is_terminal: &impl Fn(&T) -> bool,
) -> Result<bool, AppError>
where
    T: for<'de> serde::Deserialize<'de> + serde::Serialize + Clone + Send + 'static,
{
    let remaining = buffer.trim().to_string();
    buffer.clear();
    if remaining.is_empty() {
        return Ok(false);
    }

    match serde_json::from_str::<T>(&remaining) {
        Ok(event) => {
            let terminal = is_terminal(&event);
            on_event
                .send(event)
                .map_err(|e| AppError::Internal(format!("channel send error: {e}")))?;
            Ok(terminal)
        }
        Err(_) => Ok(false), // Partial line — silently discard
    }
}

// ── HTTP Client ────────────────────────────────────────────────────────────

pub struct SpritesClient {
    base_url: String,
    token: String,
    http: Client,
}

/// Convert a reqwest error into a user-friendly message with sprite context.
fn reqwest_err(e: &reqwest::Error, context: &str) -> AppError {
    if e.is_timeout() {
        AppError::Internal(format!(
            "{context}: request timed out — sprite may be unresponsive"
        ))
    } else if e.is_connect() {
        AppError::Internal(format!(
            "{context}: connection failed — sprite may be offline"
        ))
    } else if e.is_body() {
        AppError::Internal(format!(
            "{context}: connection lost while reading response"
        ))
    } else {
        AppError::Internal(format!("{context}: {e}"))
    }
}

impl SpritesClient {
    pub fn new(base_url: String, token: String) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .tcp_keepalive(Duration::from_secs(15))
            .pool_max_idle_per_host(4)
            .connection_verbose(cfg!(debug_assertions))
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

    // ── Sprites CRUD ─────────────────────────────────────────────────────

    pub async fn list_sprites(&self) -> Result<Vec<SpriteInfo>, AppError> {
        let resp = self
            .http
            .get(self.api_url("/sprites"))
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| reqwest_err(&e, "list sprites"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "list sprites returned {status}: {body}"
            )));
        }

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

    pub async fn get_sprite(&self, name: &str) -> Result<SpriteDetail, AppError> {
        let resp = self
            .http
            .get(self.api_url(&format!("/sprites/{name}")))
            .bearer_auth(&self.token)
            .timeout(LIST_TIMEOUT)
            .send()
            .await
            .map_err(|e| reqwest_err(&e, &format!("get sprite '{name}'")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "sprite detail API returned {status} for '{name}': {body}"
            )));
        }

        resp.json::<SpriteDetail>()
            .await
            .map_err(|e| AppError::Internal(format!(
                "sprite detail parse error for '{name}': {e}"
            )))
    }

    pub async fn create_sprite(&self, name: &str) -> Result<SpriteInfo, AppError> {
        let resp = self
            .http
            .post(self.api_url("/sprites"))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "name": name }))
            .send()
            .await
            .map_err(|e| reqwest_err(&e, &format!("create sprite '{name}'")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "create sprite '{name}' returned {status}: {body}"
            )));
        }

        resp.json::<SpriteInfo>()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse sprite info: {e}")))
    }

    pub async fn update_sprite(
        &self,
        name: &str,
        url_auth: &str,
    ) -> Result<SpriteDetail, AppError> {
        let resp = self
            .http
            .put(self.api_url(&format!("/sprites/{name}")))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "url_settings": { "auth": url_auth } }))
            .send()
            .await
            .map_err(|e| reqwest_err(&e, &format!("update sprite '{name}'")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "update sprite '{name}' returned {status}: {body}"
            )));
        }

        resp.json::<SpriteDetail>()
            .await
            .map_err(|e| AppError::Internal(format!("update sprite '{name}' parse error: {e}")))
    }

    pub async fn delete_sprite(&self, name: &str) -> Result<(), AppError> {
        let resp = self
            .http
            .delete(self.api_url(&format!("/sprites/{name}")))
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| reqwest_err(&e, &format!("delete sprite '{name}'")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "delete sprite '{name}' returned {status}: {body}"
            )));
        }
        Ok(())
    }

    // ── Exec ──────────────────────────────────────────────────────────────

    pub async fn exec_http(&self, name: &str, cmd: &str) -> Result<ExecResult, AppError> {
        let resp = self
            .http
            .post(self.api_url(&format!("/sprites/{name}/exec")))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "command": cmd }))
            .send()
            .await
            .map_err(|e| reqwest_err(&e, &format!("exec on '{name}'")))?;

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

    pub async fn list_exec_sessions(&self, name: &str) -> Result<Vec<ExecSession>, AppError> {
        let resp = self
            .http
            .get(self.api_url(&format!("/sprites/{name}/exec")))
            .bearer_auth(&self.token)
            .timeout(LIST_TIMEOUT)
            .send()
            .await
            .map_err(|e| reqwest_err(&e, &format!("list sessions for '{name}'")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "exec sessions API returned {status} for '{name}': {body}"
            )));
        }

        // API returns {"count": N, "sessions": [...]}
        #[derive(Deserialize)]
        struct ExecSessionsResponse {
            sessions: Vec<ExecSession>,
        }

        let body = resp
            .text()
            .await
            .map_err(|e| AppError::Internal(format!("exec sessions read error for '{name}': {e}")))?;

        // Try wrapped format first, then bare array as fallback
        if let Ok(wrapped) = serde_json::from_str::<ExecSessionsResponse>(&body) {
            return Ok(wrapped.sessions);
        }
        serde_json::from_str::<Vec<ExecSession>>(&body)
            .map_err(|e| AppError::Internal(format!(
                "exec sessions parse error for '{name}': {e}"
            )))
    }

    /// Kill exec session — returns raw Response for NDJSON streaming
    pub async fn kill_exec_session_stream(
        &self,
        name: &str,
        session_id: &str,
        signal: &str,
    ) -> Result<reqwest::Response, AppError> {
        let mut url = self.api_url(&format!("/sprites/{name}/exec/{session_id}/kill"));
        url = format!("{url}?signal={signal}");

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.token)
            .timeout(STREAM_TIMEOUT)
            .send()
            .await
            .map_err(|e| reqwest_err(&e, &format!("kill session on '{name}'")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Kill exec returned {status}: {body}"
            )));
        }
        Ok(resp)
    }

    // ── Checkpoints ──────────────────────────────────────────────────────

    pub async fn list_checkpoints(&self, name: &str) -> Result<Vec<Checkpoint>, AppError> {
        let resp = self
            .http
            .get(self.api_url(&format!("/sprites/{name}/checkpoints")))
            .bearer_auth(&self.token)
            .timeout(LIST_TIMEOUT)
            .send()
            .await
            .map_err(|e| reqwest_err(&e, &format!("list checkpoints for '{name}'")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "checkpoints API returned {status} for '{name}': {body}"
            )));
        }

        resp.json::<Vec<Checkpoint>>()
            .await
            .map_err(|e| AppError::Internal(format!(
                "checkpoints parse error for '{name}': {e}"
            )))
    }

    /// Create checkpoint — returns raw Response for NDJSON streaming
    pub async fn create_checkpoint_stream(
        &self,
        name: &str,
        comment: Option<&str>,
    ) -> Result<reqwest::Response, AppError> {
        let body = match comment {
            Some(c) => serde_json::json!({ "comment": c }),
            None => serde_json::json!({}),
        };

        let resp = self
            .http
            .post(self.api_url(&format!("/sprites/{name}/checkpoint")))
            .bearer_auth(&self.token)
            .timeout(STREAM_TIMEOUT)
            .json(&body)
            .send()
            .await
            .map_err(|e| reqwest_err(&e, &format!("create checkpoint for '{name}'")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "create checkpoint for '{name}' returned {status}: {body}"
            )));
        }
        Ok(resp)
    }

    /// Restore checkpoint — returns raw Response for NDJSON streaming
    pub async fn restore_checkpoint_stream(
        &self,
        name: &str,
        checkpoint_id: &str,
    ) -> Result<reqwest::Response, AppError> {
        let resp = self
            .http
            .post(self.api_url(&format!(
                "/sprites/{name}/checkpoints/{checkpoint_id}/restore"
            )))
            .bearer_auth(&self.token)
            .timeout(STREAM_TIMEOUT)
            .send()
            .await
            .map_err(|e| reqwest_err(&e, &format!("restore checkpoint on '{name}'")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "restore checkpoint on '{name}' returned {status}: {body}"
            )));
        }
        Ok(resp)
    }

    // ── Services ─────────────────────────────────────────────────────────

    pub async fn list_services(&self, name: &str) -> Result<Vec<Service>, AppError> {
        let resp = self
            .http
            .get(self.api_url(&format!("/sprites/{name}/services")))
            .bearer_auth(&self.token)
            .timeout(LIST_TIMEOUT)
            .send()
            .await
            .map_err(|e| reqwest_err(&e, &format!("list services for '{name}'")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "services API returned {status} for '{name}': {body}"
            )));
        }

        resp.json::<Vec<Service>>()
            .await
            .map_err(|e| AppError::Internal(format!(
                "services parse error for '{name}': {e}"
            )))
    }

    /// Start service — returns raw Response for NDJSON streaming
    pub async fn start_service_stream(
        &self,
        name: &str,
        service_name: &str,
    ) -> Result<reqwest::Response, AppError> {
        let resp = self
            .http
            .post(self.api_url(&format!(
                "/sprites/{name}/services/{service_name}/start"
            )))
            .bearer_auth(&self.token)
            .timeout(STREAM_TIMEOUT)
            .send()
            .await
            .map_err(|e| reqwest_err(&e, &format!("start service '{service_name}' on '{name}'")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Start service returned {status}: {body}"
            )));
        }
        Ok(resp)
    }

    /// Stop service — returns raw Response for NDJSON streaming
    pub async fn stop_service_stream(
        &self,
        name: &str,
        service_name: &str,
    ) -> Result<reqwest::Response, AppError> {
        let resp = self
            .http
            .post(self.api_url(&format!(
                "/sprites/{name}/services/{service_name}/stop"
            )))
            .bearer_auth(&self.token)
            .timeout(STREAM_TIMEOUT)
            .send()
            .await
            .map_err(|e| reqwest_err(&e, &format!("stop service '{service_name}' on '{name}'")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Stop service returned {status}: {body}"
            )));
        }
        Ok(resp)
    }

    /// Get service logs — returns raw Response for NDJSON streaming
    pub async fn get_service_logs_stream(
        &self,
        name: &str,
        service_name: &str,
        lines: Option<u32>,
    ) -> Result<reqwest::Response, AppError> {
        let mut url = self.api_url(&format!(
            "/sprites/{name}/services/{service_name}/logs"
        ));
        if let Some(n) = lines {
            url = format!("{url}?lines={n}");
        }

        let resp = self
            .http
            .get(&url)
            .bearer_auth(&self.token)
            .timeout(STREAM_TIMEOUT)
            .send()
            .await
            .map_err(|e| reqwest_err(&e, &format!("get logs for '{service_name}' on '{name}'")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Get service logs returned {status}: {body}"
            )));
        }
        Ok(resp)
    }

    // ── WebSocket + Config ───────────────────────────────────────────────

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

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub async fn test_connection(&self) -> Result<String, AppError> {
        let sprites = self.list_sprites().await?;
        Ok(format!("Connected. Found {} sprites.", sprites.len()))
    }

}
