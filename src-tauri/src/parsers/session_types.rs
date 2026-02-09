use serde::{Deserialize, Serialize};

/// Session status derived from JSONL entries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Thinking,
    ExecutingTool { name: String },
    AwaitingApproval,
    Waiting,
    Idle,
    Stopped,
    Unknown,
}

impl SessionStatus {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Thinking => "Thinking",
            Self::ExecutingTool { .. } => "Executing Tool",
            Self::AwaitingApproval => "Awaiting Approval",
            Self::Waiting => "Waiting for Input",
            Self::Idle => "Idle",
            Self::Stopped => "Stopped",
            Self::Unknown => "Unknown",
        }
    }

    pub fn color(&self) -> &str {
        match self {
            Self::Thinking => "blue",
            Self::ExecutingTool { .. } => "orange",
            Self::AwaitingApproval => "yellow",
            Self::Waiting => "blue",
            Self::Idle => "gray",
            Self::Stopped => "red",
            Self::Unknown => "gray",
        }
    }
}

/// Session info returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub project_path: String,
    pub encoded_path: String,
    pub jsonl_path: String,
    pub last_modified: u64,
    pub status: SessionStatus,
    pub model: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_output_tokens: u64,
    pub git_branch: Option<String>,
    pub cwd: Option<String>,
}

/// Activity entry for the recent activity list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub timestamp: u64,
    pub activity_type: ActivityType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    ToolUse { name: String },
    ToolResult { name: String, success: bool },
    UserMessage,
    AssistantMessage,
    Thinking,
}

/// Raw JSONL entry as it appears in Claude Code session files
#[derive(Debug, Deserialize)]
pub struct JsonlEntry {
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
    pub message: Option<JsonlMessage>,
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,
    pub uuid: Option<String>,
    pub timestamp: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    #[serde(rename = "gitBranch")]
    pub git_branch: Option<String>,
    pub cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct JsonlMessage {
    pub role: Option<String>,
    pub content: Option<serde_json::Value>,
    pub usage: Option<JsonlUsage>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct JsonlUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}
