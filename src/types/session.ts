export type SessionStatus =
    | { type: "thinking" }
    | { type: "executing_tool"; name: string }
    | { type: "awaiting_approval" }
    | { type: "waiting" }
    | { type: "idle" }
    | { type: "stopped" }
    | { type: "unknown" };

export interface SessionInfo {
    id: string;
    project_path: string;
    encoded_path: string;
    jsonl_path: string;
    last_modified: number;
    status: SessionStatus;
    model: string | null;
    input_tokens: number;
    output_tokens: number;
    total_output_tokens: number;
    context_tokens: number;
    cache_creation_tokens: number;
    cache_read_tokens: number;
    git_branch: string | null;
    cwd: string | null;
}

export interface ActivityEntry {
    timestamp: number;
    activity_type: ActivityType;
    description: string;
}

export type ActivityType =
    | { type: "tool_use"; name: string }
    | { type: "tool_result"; name: string; success: boolean }
    | { type: "user_message" }
    | { type: "assistant_message" }
    | { type: "thinking" };

export function statusDisplayName(status: SessionStatus): string {
    switch (status.type) {
        case "thinking":
            return "Thinking";
        case "executing_tool":
            return `Executing: ${status.name}`;
        case "awaiting_approval":
            return "Awaiting Approval";
        case "waiting":
            return "Waiting for Input";
        case "idle":
            return "Idle";
        case "stopped":
            return "Stopped";
        case "unknown":
            return "Unknown";
    }
}

export function statusColor(status: SessionStatus): string {
    switch (status.type) {
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
        case "unknown":
            return "text-gray-500";
    }
}

export function statusDotColor(status: SessionStatus): string {
    switch (status.type) {
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
        case "unknown":
            return "bg-gray-500";
    }
}

export function isActiveStatus(status: SessionStatus): boolean {
    return status.type === "thinking" || status.type === "executing_tool";
}
