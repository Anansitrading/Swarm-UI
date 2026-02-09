export interface TeamMember {
    agentId: string;
    name: string;
    agentType: string;
    model?: string;
    joinedAt?: number;
    tmuxPaneId?: string;
    cwd?: string;
}

export interface TeamTask {
    id: string;
    subject?: string;
    description?: string;
    activeForm?: string;
    owner?: string;
    status?: string;
    blocks: string[];
    blockedBy: string[];
}

export interface TaskSummary {
    total: number;
    pending: number;
    in_progress: number;
    completed: number;
}

export interface TeamInfo {
    name: string;
    description?: string;
    createdAt?: number;
    leadAgentId?: string;
    leadSessionId?: string;
    members: TeamMember[];
    tasks: TeamTask[];
    taskSummary: TaskSummary;
    hasInboxes: boolean;
}

export function taskStatusColor(status?: string): string {
    switch (status) {
        case "completed":
            return "text-green-400";
        case "in_progress":
        case "in-progress":
            return "text-orange-400";
        case "pending":
            return "text-gray-400";
        default:
            return "text-gray-500";
    }
}

export function taskStatusDot(status?: string): string {
    switch (status) {
        case "completed":
            return "bg-green-400";
        case "in_progress":
        case "in-progress":
            return "bg-orange-400";
        case "pending":
            return "bg-gray-400";
        default:
            return "bg-gray-500";
    }
}

export function taskStatusLabel(status?: string): string {
    switch (status) {
        case "completed":
            return "Done";
        case "in_progress":
        case "in-progress":
            return "Active";
        case "pending":
            return "Pending";
        default:
            return "Unknown";
    }
}
