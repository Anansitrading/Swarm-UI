import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, useCallback } from "react";
import type { SessionInfo } from "../../types/session";
import { StatusBadge } from "./StatusBadge";
import { ContextBar } from "./ContextBar";

interface SessionDetailProps {
  session: SessionInfo;
  onOpenTerminal: (cwd: string) => void;
  onKillProcess: (pid: number) => void;
}

interface ProcessInfo {
  pid: number;
  cmdline: string;
  cwd: string;
}

export function SessionDetail({ session, onOpenTerminal, onKillProcess }: SessionDetailProps) {
  const [detail, setDetail] = useState<SessionInfo>(session);
  const [processes, setProcesses] = useState<ProcessInfo[]>([]);

  const refresh = useCallback(async () => {
    try {
      const info = await invoke<SessionInfo>("get_session_detail", {
        jsonlPath: session.jsonl_path,
      });
      setDetail(info);
    } catch {
      // Keep current data on error
    }
  }, [session.jsonl_path]);

  const fetchProcesses = useCallback(async () => {
    try {
      const procs = await invoke<ProcessInfo[]>("find_claude_processes");
      // Filter to processes matching this session's CWD
      const cwd = session.cwd || session.project_path;
      const matched = procs.filter((p) => p.cwd.includes(cwd) || cwd.includes(p.cwd));
      setProcesses(matched);
    } catch {
      setProcesses([]);
    }
  }, [session.cwd, session.project_path]);

  useEffect(() => {
    refresh();
    fetchProcesses();
    const interval = setInterval(() => {
      refresh();
      fetchProcesses();
    }, 5000);
    return () => clearInterval(interval);
  }, [refresh, fetchProcesses]);

  // Update detail when session prop changes
  useEffect(() => {
    setDetail(session);
  }, [session]);

  const projectName = detail.project_path.split("/").pop() || detail.project_path;
  const cwd = detail.cwd || detail.project_path;

  return (
    <div className="flex flex-col h-full bg-swarm-surface overflow-y-auto">
      {/* Header */}
      <div className="p-4 border-b border-swarm-border">
        <div className="flex items-center justify-between mb-2">
          <h2 className="text-lg font-semibold text-swarm-text">{projectName}</h2>
          <StatusBadge status={detail.status} />
        </div>
        <div className="text-xs text-swarm-text-dim font-mono">{cwd}</div>
        {detail.git_branch && (
          <div className="text-xs text-swarm-text-dim mt-1 flex items-center gap-1">
            <svg className="w-3 h-3" viewBox="0 0 16 16" fill="currentColor">
              <path d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.5 2.5 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25Z" />
            </svg>
            {detail.git_branch}
          </div>
        )}
      </div>

      {/* Stats */}
      <div className="grid grid-cols-3 gap-3 p-4 border-b border-swarm-border">
        <div>
          <div className="text-xs text-swarm-text-dim">Model</div>
          <div className="text-sm text-swarm-text font-medium">
            {detail.model ? formatModel(detail.model) : "-"}
          </div>
        </div>
        <div>
          <div className="text-xs text-swarm-text-dim">Input</div>
          <div className="text-sm text-swarm-text font-mono">
            {formatTokens(detail.input_tokens)}
          </div>
        </div>
        <div>
          <div className="text-xs text-swarm-text-dim">Total Output</div>
          <div className="text-sm text-swarm-text font-mono">
            {formatTokens(detail.total_output_tokens)}
          </div>
        </div>
      </div>

      {/* Context Usage */}
      {detail.input_tokens > 0 && (
        <div className="p-4 border-b border-swarm-border">
          <div className="text-xs text-swarm-text-dim mb-2">Context Window</div>
          <ContextBar inputTokens={detail.input_tokens} model={detail.model} />
        </div>
      )}

      {/* Actions */}
      <div className="p-4 border-b border-swarm-border space-y-2">
        <button
          onClick={() => onOpenTerminal(cwd)}
          className="w-full px-3 py-2 text-sm bg-swarm-accent/20 text-swarm-accent border border-swarm-accent/30 rounded hover:bg-swarm-accent/30 transition-colors"
        >
          Open Terminal in {projectName}
        </button>
      </div>

      {/* Processes */}
      {processes.length > 0 && (
        <div className="p-4">
          <div className="text-xs text-swarm-text-dim mb-2">Processes ({processes.length})</div>
          <div className="space-y-2">
            {processes.map((proc) => (
              <div
                key={proc.pid}
                className="flex items-center justify-between p-2 rounded bg-swarm-bg border border-swarm-border text-xs"
              >
                <div className="min-w-0">
                  <div className="text-swarm-text font-mono">PID {proc.pid}</div>
                  <div className="text-swarm-text-dim truncate">{proc.cmdline}</div>
                </div>
                <button
                  onClick={() => onKillProcess(proc.pid)}
                  className="ml-2 px-2 py-1 text-red-400 border border-red-400/30 rounded hover:bg-red-400/10 transition-colors shrink-0"
                >
                  Kill
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Session ID */}
      <div className="p-4 mt-auto">
        <div className="text-xs text-swarm-text-dim">
          Session: <span className="font-mono">{detail.id.slice(0, 8)}</span>
        </div>
      </div>
    </div>
  );
}

function formatModel(model: string): string {
  const lower = model.toLowerCase();
  if (lower.includes("opus")) return "Opus 4.6";
  if (lower.includes("sonnet")) return "Sonnet 4.5";
  if (lower.includes("haiku")) return "Haiku 4.5";
  return model.split("-").slice(0, 2).join(" ");
}

function formatTokens(n: number): string {
  if (n === 0) return "-";
  if (n < 1000) return String(n);
  if (n < 1_000_000) return `${(n / 1000).toFixed(1)}K`;
  return `${(n / 1_000_000).toFixed(2)}M`;
}
