use crate::error::AppError;
use crate::state::PtyInfo;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite;

/// A WebSocket session connected to a Sprite's exec endpoint
pub struct WsSession {
    pub id: String,
    pub sprite_name: String,
    pub tx: Arc<
        Mutex<
            futures_util::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
                tungstenite::Message,
            >,
        >,
    >,
    pub cols: u16,
    pub rows: u16,
    _abort: tokio::task::AbortHandle,
}

/// Shared state for WebSocket sessions
pub struct WsState {
    pub sessions: Mutex<HashMap<String, WsSession>>,
}

impl WsState {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

/// Connect to a sprite's exec WebSocket for interactive terminal
pub async fn sprite_ws_connect(
    sprite_name: &str,
    ws_url: &str,
    token: &str,
    cols: u16,
    rows: u16,
    app: AppHandle,
    ws_state: &WsState,
) -> Result<PtyInfo, AppError> {
    let session_id = uuid::Uuid::new_v4().to_string();

    // Build WebSocket request with auth header
    let request = tungstenite::http::Request::builder()
        .uri(ws_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Host", extract_host(ws_url))
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header(
            "Sec-WebSocket-Key",
            tungstenite::handshake::client::generate_key(),
        )
        .body(())
        .map_err(|e| AppError::Internal(format!("Failed to build WS request: {e}")))?;

    let (ws_stream, _response) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| AppError::Internal(format!("WebSocket connection failed: {e}")))?;

    let (write, mut read) = ws_stream.split();
    let write = Arc::new(Mutex::new(write));

    // Spawn reader task that emits pty:data events (same format as local PTY)
    let sid = session_id.clone();
    let app_clone = app.clone();
    let reader_task = tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(tungstenite::Message::Binary(data)) => {
                    let b64 =
                        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                    let _ = app_clone.emit(&format!("pty:data:{}", sid), b64);
                }
                Ok(tungstenite::Message::Text(text)) => {
                    let b64 = base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        text.as_bytes(),
                    );
                    let _ = app_clone.emit(&format!("pty:data:{}", sid), b64);
                }
                Ok(tungstenite::Message::Close(_)) => {
                    let _ = app_clone.emit(&format!("pty:exit:{}", sid), 0);
                    break;
                }
                Err(e) => {
                    tracing::error!("WebSocket read error for {}: {}", sid, e);
                    let _ = app_clone.emit(&format!("pty:exit:{}", sid), 1);
                    break;
                }
                _ => {} // Ping/Pong handled automatically
            }
        }
    });

    let abort_handle = reader_task.abort_handle();

    let ws_session = WsSession {
        id: session_id.clone(),
        sprite_name: sprite_name.to_string(),
        tx: write,
        cols,
        rows,
        _abort: abort_handle,
    };

    ws_state
        .sessions
        .lock()
        .await
        .insert(session_id.clone(), ws_session);

    Ok(PtyInfo {
        id: session_id,
        pid: 0, // No local PID for remote sessions
        cols,
        rows,
    })
}

/// Write data to a WebSocket session
pub async fn ws_write(session_id: &str, data: &[u8], ws_state: &WsState) -> Result<(), AppError> {
    let sessions = ws_state.sessions.lock().await;
    let session = sessions
        .get(session_id)
        .ok_or_else(|| AppError::NotFound(format!("WS session {} not found", session_id)))?;

    let mut tx = session.tx.lock().await;
    tx.send(tungstenite::Message::Binary(data.to_vec().into()))
        .await
        .map_err(|e| AppError::Internal(format!("WebSocket write failed: {e}")))?;

    Ok(())
}

/// Resize a WebSocket session (send resize escape sequence)
pub async fn ws_resize(
    session_id: &str,
    cols: u16,
    rows: u16,
    ws_state: &WsState,
) -> Result<(), AppError> {
    let mut sessions = ws_state.sessions.lock().await;
    let session = sessions
        .get_mut(session_id)
        .ok_or_else(|| AppError::NotFound(format!("WS session {} not found", session_id)))?;

    session.cols = cols;
    session.rows = rows;

    // Send resize as JSON message (Sprites API protocol)
    let resize_msg = serde_json::json!({
        "type": "resize",
        "cols": cols,
        "rows": rows
    });

    let mut tx = session.tx.lock().await;
    tx.send(tungstenite::Message::Text(resize_msg.to_string()))
        .await
        .map_err(|e| AppError::Internal(format!("WebSocket resize failed: {e}")))?;

    Ok(())
}

/// Kill a WebSocket session
pub async fn ws_kill(session_id: &str, ws_state: &WsState) -> Result<(), AppError> {
    let mut sessions = ws_state.sessions.lock().await;
    if let Some(session) = sessions.remove(session_id) {
        // Close the WebSocket
        let mut tx = session.tx.lock().await;
        let _ = tx.send(tungstenite::Message::Close(None)).await;
        // Abort handle drops automatically, killing the reader task
    }
    Ok(())
}

fn extract_host(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("api.sprites.dev")
        .to_string()
}
