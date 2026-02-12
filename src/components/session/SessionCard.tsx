import type { SessionInfo } from "../../types/session";

interface SessionCardProps {
    session: SessionInfo;
    selected: boolean;
    onClick: () => void;
}

export function SessionCard({ session, selected, onClick }: SessionCardProps) {
    const timeAgo = formatTimeAgo(session.last_modified);
    const isActive =
        session.status.type === "thinking" ||
        session.status.type === "executing_tool";

    return (
        <button
            onClick={onClick}
            className={`w-full text-left px-3 py-1.5 ml-4 mr-1 border-l-2 transition-colors ${
                selected
                    ? "border-swarm-accent bg-swarm-accent/10"
                    : isActive
                      ? "border-green-400/50 hover:bg-swarm-accent/5"
                      : "border-transparent hover:border-swarm-border hover:bg-swarm-accent/5"
            }`}
        >
            <div className="flex items-center gap-2">
                {/* Git branch icon + name */}
                <svg
                    className="w-3 h-3 text-swarm-text-dim shrink-0"
                    viewBox="0 0 16 16"
                    fill="currentColor"
                >
                    <path d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.5 2.5 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25Z" />
                </svg>
                <span className="text-xs text-swarm-text truncate">
                    {session.git_branch || session.id.slice(0, 8)}
                </span>

                {/* Session count / message count badges */}
                {session.input_tokens > 0 && (
                    <span className="text-[9px] text-swarm-text-dim tabular-nums shrink-0">
                        {formatTokensShort(session.input_tokens)}
                    </span>
                )}

                {/* Status dot */}
                <span className="ml-auto shrink-0">
                    <StatusDot status={session.status} />
                </span>
            </div>

            {/* Session ID for traceability */}
            <div className="mt-0.5 pl-5">
                <span className="text-[9px] font-mono text-swarm-text-dim/60 select-all">
                    {session.id}
                </span>
            </div>

            {/* Model + time */}
            <div className="flex items-center justify-between mt-0.5 pl-5">
                {session.model && (
                    <span className="text-[10px] text-swarm-text-dim font-mono">
                        {formatModel(session.model)}
                    </span>
                )}
                <span className="text-[10px] text-swarm-text-dim">
                    {timeAgo}
                </span>
            </div>

            {/* Context bar - mini version */}
            {session.input_tokens > 0 && (
                <div className="mt-1 pl-5">
                    <div className="h-0.5 bg-swarm-border rounded-full overflow-hidden">
                        <div
                            className={`h-full rounded-full transition-all ${
                                session.input_tokens / 200000 > 0.8
                                    ? "bg-red-500"
                                    : session.input_tokens / 200000 > 0.6
                                      ? "bg-yellow-500"
                                      : "bg-swarm-accent"
                            }`}
                            style={{
                                width: `${Math.min((session.input_tokens / 200000) * 100, 100)}%`,
                            }}
                        />
                    </div>
                </div>
            )}
        </button>
    );
}

function StatusDot({ status }: { status: SessionInfo["status"] }) {
    const colors: Record<string, string> = {
        thinking: "bg-blue-400",
        executing_tool: "bg-orange-400",
        awaiting_approval: "bg-yellow-400",
        waiting: "bg-blue-300",
        idle: "bg-gray-400",
        stopped: "bg-red-400",
        unknown: "bg-gray-500",
    };
    const color = colors[status.type] || "bg-gray-500";
    const isActive =
        status.type === "thinking" || status.type === "executing_tool";

    return (
        <span className="relative flex h-1.5 w-1.5">
            {isActive && (
                <span
                    className={`animate-ping absolute inline-flex h-full w-full rounded-full ${color} opacity-75`}
                />
            )}
            <span
                className={`relative inline-flex rounded-full h-1.5 w-1.5 ${color}`}
            />
        </span>
    );
}

function formatModel(model: string): string {
    const lower = model.toLowerCase();
    if (lower.includes("opus")) return "Opus";
    if (lower.includes("sonnet")) return "Sonnet";
    if (lower.includes("haiku")) return "Haiku";
    return model.split("-")[0] ?? model;
}

function formatTokensShort(n: number): string {
    if (n >= 1000) return `${(n / 1000).toFixed(0)}K`;
    return String(n);
}

function formatTimeAgo(epochSec: number): string {
    if (!epochSec) return "";
    const now = Date.now() / 1000;
    const diff = now - epochSec;
    if (diff < 60) return "just now";
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
}
