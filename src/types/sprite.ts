export interface SpriteInfo {
    name: string;
    status: string;
    id?: string;
    region?: string;
}

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
