import { useSessionStore } from "../stores/sessionStore";
import type { SessionInfo } from "../types/session";

/**
 * Hook to subscribe to a specific session's data.
 * Returns the session info and auto-updates when the session changes.
 */
export function useSession(sessionId: string | null): SessionInfo | null {
  const sessions = useSessionStore((s) => s.sessions);

  if (!sessionId) return null;
  return sessions.find((s) => s.id === sessionId) ?? null;
}
