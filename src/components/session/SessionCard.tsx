import type { SessionInfo } from "../../types/session";
import { StatusBadge } from "./StatusBadge";
import { ContextBar } from "./ContextBar";

interface SessionCardProps {
  session: SessionInfo;
  selected: boolean;
  onClick: () => void;
}

export function SessionCard({ session, selected, onClick }: SessionCardProps) {
  const projectName = session.project_path.split("/").pop() || session.project_path;
  const timeAgo = formatTimeAgo(session.last_modified);

  return (
    <button
      onClick={onClick}
      className={`w-full text-left p-3 rounded-lg border transition-colors ${
        selected
          ? "bg-swarm-accent/10 border-swarm-accent/30"
          : "bg-swarm-surface border-swarm-border hover:border-swarm-accent/20"
      }`}
    >
      <div className="flex items-start justify-between gap-2 mb-2">
        <div className="min-w-0">
          <div className="font-medium text-sm text-swarm-text truncate">
            {projectName}
          </div>
          {session.git_branch && (
            <div className="text-xs text-swarm-text-dim mt-0.5 flex items-center gap-1">
              <svg className="w-3 h-3" viewBox="0 0 16 16" fill="currentColor">
                <path d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.5 2.5 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25Z" />
              </svg>
              {session.git_branch}
            </div>
          )}
        </div>
        <StatusBadge status={session.status} />
      </div>

      {session.input_tokens > 0 && (
        <ContextBar inputTokens={session.input_tokens} model={session.model} />
      )}

      <div className="flex items-center justify-between mt-2 text-xs text-swarm-text-dim">
        {session.model && (
          <span className="font-mono">{formatModel(session.model)}</span>
        )}
        <span>{timeAgo}</span>
      </div>
    </button>
  );
}

function formatModel(model: string): string {
  const lower = model.toLowerCase();
  if (lower.includes("opus")) return "Opus";
  if (lower.includes("sonnet")) return "Sonnet";
  if (lower.includes("haiku")) return "Haiku";
  return model.split("-")[0] ?? model;
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
