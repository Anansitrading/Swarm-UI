use serde::{Deserialize, Serialize};

/// Filter for `list_sessions` command.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SessionFilter {
    pub project: Option<String>,
    pub git_branch: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub include_archived: bool,
}

/// Filter for `search_sessions` command.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchFilter {
    pub project: Option<String>,
    #[serde(default)]
    pub include_tool_output: bool,
    pub limit: Option<usize>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub role: Option<String>,
}

impl Default for SearchFilter {
    fn default() -> Self {
        Self {
            project: None,
            include_tool_output: false,
            limit: None,
            date_from: None,
            date_to: None,
            role: None,
        }
    }
}

/// Single session entry returned by `list_sessions`.
#[derive(Debug, Clone, Serialize)]
pub struct SessionListItem {
    pub session_id: String,
    pub project_path: String,
    pub summary: String,
    pub first_prompt: String,
    pub git_branch: String,
    pub model: String,
    pub status: String,
    pub message_count: u64,
    pub total_tokens: u64,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub has_tool_use: bool,
    pub file_exists: bool,
    pub archived: bool,
}

/// A matched snippet within a search result.
#[derive(Debug, Clone, Serialize)]
pub struct MatchSnippet {
    pub role: String,
    pub content_type: String,
    pub snippet: String,
    pub timestamp: Option<String>,
    pub turn_index: u64,
}

/// Single result from `search_sessions`.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub session_id: String,
    pub score: f32,
    pub snippets: Vec<MatchSnippet>,
    pub project_path: Option<String>,
    pub summary: Option<String>,
    pub model: Option<String>,
    pub modified_at: Option<String>,
    pub file_exists: bool,
}

/// Full session metadata returned by `get_session_detail`.
#[derive(Debug, Clone, Serialize)]
pub struct SessionDetail {
    pub session_id: String,
    pub project_path: String,
    pub summary: String,
    pub first_prompt: String,
    pub git_branch: String,
    pub model: String,
    pub status: String,
    pub jsonl_path: String,
    pub message_count: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub has_tool_use: bool,
    pub file_exists: bool,
    pub archived: bool,
    pub turn_depth: u64,
}

/// A single message in a conversation, returned by `get_conversation`.
#[derive(Debug, Clone, Serialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content_type: String,
    pub text: String,
    pub timestamp: Option<String>,
    pub truncated: bool,
}

/// Index statistics returned by `get_index_stats`.
#[derive(Debug, Clone, Serialize)]
pub struct IndexStats {
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub archived_sessions: u64,
    pub total_messages: u64,
    pub segment_count: u64,
    pub index_size_bytes: u64,
}

/// Progress event payload emitted during bulk indexing.
#[derive(Debug, Clone, Serialize)]
pub struct IndexProgress {
    pub phase: String,
    pub current: u64,
    pub total: u64,
}

/// On-disk metadata stored in `swarm-ui-meta.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMeta {
    pub schema_version: u64,
    pub indexed_at: String,
    pub session_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_filter_defaults() {
        let filter = SessionFilter::default();
        assert!(filter.project.is_none());
        assert!(filter.git_branch.is_none());
        assert!(filter.model.is_none());
        assert!(!filter.include_archived);
    }

    #[test]
    fn session_filter_deserializes_from_json() {
        let json = r#"{"project": "swarm-ui", "include_archived": true}"#;
        let filter: SessionFilter = serde_json::from_str(json).unwrap();
        assert_eq!(filter.project.as_deref(), Some("swarm-ui"));
        assert!(filter.include_archived);
        assert!(filter.git_branch.is_none());
    }

    #[test]
    fn session_filter_deserializes_empty_json() {
        let json = r#"{}"#;
        let filter: SessionFilter = serde_json::from_str(json).unwrap();
        assert!(!filter.include_archived);
    }

    #[test]
    fn search_filter_defaults() {
        let filter = SearchFilter::default();
        assert!(filter.project.is_none());
        assert!(!filter.include_tool_output);
        assert!(filter.limit.is_none());
        assert!(filter.date_from.is_none());
        assert!(filter.date_to.is_none());
        assert!(filter.role.is_none());
    }

    #[test]
    fn search_filter_deserializes_from_json() {
        let json = r#"{
            "project": "swarm-ui",
            "include_tool_output": true,
            "limit": 50,
            "date_from": "2026-01-01",
            "date_to": "2026-02-18",
            "role": "user"
        }"#;
        let filter: SearchFilter = serde_json::from_str(json).unwrap();
        assert_eq!(filter.project.as_deref(), Some("swarm-ui"));
        assert!(filter.include_tool_output);
        assert_eq!(filter.limit, Some(50));
        assert_eq!(filter.date_from.as_deref(), Some("2026-01-01"));
        assert_eq!(filter.date_to.as_deref(), Some("2026-02-18"));
        assert_eq!(filter.role.as_deref(), Some("user"));
    }

    #[test]
    fn session_list_item_serializes_to_json() {
        let item = SessionListItem {
            session_id: "abc-123".into(),
            project_path: "/home/user/project".into(),
            summary: "Implement feature X".into(),
            first_prompt: "Help me build X".into(),
            git_branch: "main".into(),
            model: "claude-opus-4-6".into(),
            status: "idle".into(),
            message_count: 42,
            total_tokens: 9001,
            created_at: Some("2026-02-18T12:00:00Z".into()),
            modified_at: Some("2026-02-18T13:00:00Z".into()),
            has_tool_use: true,
            file_exists: true,
            archived: false,
        };
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["session_id"], "abc-123");
        assert_eq!(json["message_count"], 42);
        assert_eq!(json["has_tool_use"], true);
        assert_eq!(json["archived"], false);
    }

    #[test]
    fn search_result_serializes_with_snippets() {
        let result = SearchResult {
            session_id: "sess-1".into(),
            score: 3.14,
            snippets: vec![
                MatchSnippet {
                    role: "user".into(),
                    content_type: "text".into(),
                    snippet: "How do I implement...".into(),
                    timestamp: Some("2026-02-18T12:00:00Z".into()),
                    turn_index: 0,
                },
                MatchSnippet {
                    role: "assistant".into(),
                    content_type: "text".into(),
                    snippet: "You can use the following...".into(),
                    timestamp: None,
                    turn_index: 1,
                },
            ],
            project_path: Some("/home/user/project".into()),
            summary: Some("Feature implementation".into()),
            model: Some("claude-opus-4-6".into()),
            modified_at: Some("2026-02-18T13:00:00Z".into()),
            file_exists: true,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["snippets"].as_array().unwrap().len(), 2);
        assert_eq!(json["snippets"][0]["role"], "user");
        assert_eq!(json["snippets"][1]["turn_index"], 1);
    }

    #[test]
    fn session_detail_serializes_all_fields() {
        let detail = SessionDetail {
            session_id: "abc".into(),
            project_path: "/project".into(),
            summary: "summary".into(),
            first_prompt: "prompt".into(),
            git_branch: "main".into(),
            model: "claude-opus-4-6".into(),
            status: "idle".into(),
            jsonl_path: "/path/to/file.jsonl".into(),
            message_count: 10,
            input_tokens: 500,
            output_tokens: 1500,
            total_tokens: 2000,
            created_at: None,
            modified_at: None,
            has_tool_use: false,
            file_exists: true,
            archived: false,
            turn_depth: 5,
        };
        let json = serde_json::to_value(&detail).unwrap();
        assert_eq!(json["jsonl_path"], "/path/to/file.jsonl");
        assert_eq!(json["input_tokens"], 500);
        assert_eq!(json["turn_depth"], 5);
    }

    #[test]
    fn conversation_message_serializes() {
        let msg = ConversationMessage {
            role: "assistant".into(),
            content_type: "text".into(),
            text: "Here is the answer...".into(),
            timestamp: Some("2026-02-18T12:00:00Z".into()),
            truncated: false,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "assistant");
        assert_eq!(json["truncated"], false);

        let truncated = ConversationMessage {
            role: "user".into(),
            content_type: "text".into(),
            text: "First 500 chars...".into(),
            timestamp: None,
            truncated: true,
        };
        let json = serde_json::to_value(&truncated).unwrap();
        assert_eq!(json["truncated"], true);
    }

    #[test]
    fn index_stats_serializes() {
        let stats = IndexStats {
            total_sessions: 21169,
            active_sessions: 20000,
            archived_sessions: 1169,
            total_messages: 500000,
            segment_count: 12,
            index_size_bytes: 314_159_265,
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["total_sessions"], 21169);
        assert_eq!(json["active_sessions"], 20000);
        assert_eq!(json["index_size_bytes"], 314_159_265u64);
    }

    #[test]
    fn index_progress_serializes() {
        let progress = IndexProgress {
            phase: "parsing".into(),
            current: 5000,
            total: 21169,
        };
        let json = serde_json::to_value(&progress).unwrap();
        assert_eq!(json["phase"], "parsing");
        assert_eq!(json["current"], 5000);
        assert_eq!(json["total"], 21169);
    }

    #[test]
    fn index_meta_round_trips() {
        let meta = IndexMeta {
            schema_version: 1,
            indexed_at: "2026-02-18T12:00:00Z".into(),
            session_count: 21169,
        };
        let json_str = serde_json::to_string(&meta).unwrap();
        let parsed: IndexMeta = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.schema_version, meta.schema_version);
        assert_eq!(parsed.indexed_at, meta.indexed_at);
        assert_eq!(parsed.session_count, meta.session_count);
    }
}
