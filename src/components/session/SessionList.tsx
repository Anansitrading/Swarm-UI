import { useEffect } from "react";
import { useSessionStore } from "../../stores/sessionStore";
import { SessionCard } from "./SessionCard";

export function SessionList() {
  const { sessions, selectedSessionId, loading, fetchSessions, selectSession } =
    useSessionStore();

  useEffect(() => {
    fetchSessions();
  }, [fetchSessions]);

  if (loading && sessions.length === 0) {
    return (
      <div className="flex items-center justify-center p-8 text-swarm-text-dim text-sm">
        Loading sessions...
      </div>
    );
  }

  if (sessions.length === 0) {
    return (
      <div className="p-4 text-center text-swarm-text-dim text-sm">
        <p>No active sessions found.</p>
        <p className="mt-1 text-xs">Start a Claude Code session to see it here.</p>
      </div>
    );
  }

  return (
    <div className="space-y-2 p-2">
      {sessions.map((session) => (
        <SessionCard
          key={session.jsonl_path}
          session={session}
          selected={selectedSessionId === session.id}
          onClick={() => selectSession(session.id)}
        />
      ))}
    </div>
  );
}
