// --- IPC Data Types (Tantivy backend) ---

/** list_sessions response */
export interface SessionListItem {
    session_id: string;
    project_path: string;
    summary: string;
    first_prompt: string;
    git_branch: string;
    model: string;
    status: string;
    message_count: number;
    total_tokens: number;
    created_at?: string;
    modified_at?: string;
    has_tool_use: boolean;
    file_exists: boolean;
    archived: boolean;
}

/** search_sessions response */
export interface SearchResult {
    session_id: string;
    score: number;
    snippets: MatchSnippet[];
    project_path?: string;
    summary?: string;
    model?: string;
    modified_at?: string;
    file_exists: boolean;
}

export interface MatchSnippet {
    role: string;
    content_type: string;
    snippet: string;
    timestamp?: string;
    turn_index: number;
}

/** get_conversation response */
export interface ConversationMessage {
    role: string;
    content_type: string;
    text: string;
    timestamp?: string;
    truncated: boolean;
}

/** get_index_stats response */
export interface IndexStats {
    total_sessions: number;
    active_sessions: number;
    archived_sessions: number;
    total_messages: number;
    segment_count: number;
    index_size_bytes: number;
}

/** index:progress event payload */
export interface IndexProgress {
    phase: string;
    current: number;
    total: number;
}

/** list_sessions filter */
export interface SessionFilter {
    project?: string;
    git_branch?: string;
    model?: string;
    include_archived: boolean;
}

/** search_sessions filter */
export interface SearchFilter {
    project?: string;
    include_tool_output: boolean;
    limit?: number;
    date_from?: string;
    date_to?: string;
    role?: string;
}

// --- Status helpers (status is a plain string from Tantivy) ---

export function statusDisplayName(status: string): string {
    switch (status) {
        case "thinking":
            return "Thinking";
        case "executing_tool":
            return "Executing Tool";
        case "awaiting_approval":
            return "Awaiting Approval";
        case "waiting":
            return "Waiting for Input";
        case "idle":
            return "Idle";
        case "stopped":
            return "Stopped";
        default:
            return status || "Unknown";
    }
}

export function statusColor(status: string): string {
    switch (status) {
        case "thinking":
            return "text-blue-400";
        case "executing_tool":
            return "text-orange-400";
        case "awaiting_approval":
            return "text-yellow-400";
        case "waiting":
            return "text-blue-300";
        case "idle":
            return "text-gray-400";
        case "stopped":
            return "text-red-400";
        default:
            return "text-gray-500";
    }
}

export function statusDotColor(status: string): string {
    switch (status) {
        case "thinking":
            return "bg-blue-400";
        case "executing_tool":
            return "bg-orange-400";
        case "awaiting_approval":
            return "bg-yellow-400";
        case "waiting":
            return "bg-blue-300";
        case "idle":
            return "bg-gray-400";
        case "stopped":
            return "bg-red-400";
        default:
            return "bg-gray-500";
    }
}

export function isActiveStatus(status: string): boolean {
    return status === "thinking" || status === "executing_tool";
}
