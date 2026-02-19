// --- Sprite core types ---

export interface SpriteInfo {
    name: string;
    status: "cold" | "warm" | "running" | string;
    id?: string;
    region?: string;
}

export interface SpriteDetail {
    name: string;
    status: "cold" | "warm" | "running" | string;
    id?: string;
    organization?: string;
    url?: string;
    url_settings?: { auth: "sprite" | "public" };
    created_at?: string;
    updated_at?: string;
    last_started_at?: string;
    last_active_at?: string;
}

// --- Checkpoint types ---

export interface Checkpoint {
    id: string;
    comment?: string;
    created_at?: string;
    create_time?: string;
    source_id?: string;
}

// --- Exec session types ---

export interface ExecSession {
    id: number | string;
    command?: string;
    is_active?: boolean;
    tty?: boolean;
    created?: string;
    last_activity?: string;
    workdir?: string;
    bytes_per_second?: number;
}

// --- Service types ---

export interface ServiceState {
    name: string;
    status: "stopped" | "starting" | "running" | "stopping" | "failed";
    pid?: number;
    started_at?: string;
    error?: string;
}

export interface Service {
    name: string;
    cmd: string;
    args: string[];
    needs: string[];
    http_port?: number;
    state?: ServiceState;
}

// --- NDJSON streaming event types ---

export interface StreamEvent {
    type: "info" | "error" | "complete";
    data?: string;
    error?: string;
    time?: string;
}

export interface ServiceStreamEvent {
    type:
        | "started"
        | "stopping"
        | "stopped"
        | "stdout"
        | "stderr"
        | "error"
        | "exit"
        | "complete";
    data?: string;
    exit_code?: number;
    timestamp?: number;
    log_files?: Record<string, string>;
}

export interface ExecKillEvent {
    type: "signal" | "timeout" | "exited" | "killed" | "error" | "complete";
    message?: string;
    signal?: string;
    pid?: number;
    exit_code?: number;
}

// --- Per-entity operation state ---

export interface OperationState {
    loading: boolean;
    progress: string[];
    error: string | null;
}

// --- Introspection types (existing, used for ps aux / file scanning) ---

export interface SpriteSessionInfo {
    pid: string;
    command: string;
    status: string;
}

export interface SpriteClaudeSessionInfo {
    session_id: string;
    project_dir: string;
    jsonl_path: string;
}

export interface SpriteTeamInfo {
    name: string;
    member_count: number;
}
