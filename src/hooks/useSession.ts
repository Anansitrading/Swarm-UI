import { useSessionStore } from "../stores/sessionStore";
import type { SessionListItem } from "../types/session";

/**
 * Hook to subscribe to a specific session's data.
 * Returns the session info and auto-updates when the session changes.
 */
export function useSession(sessionId: string | null): SessionListItem | null {
  const sessions = useSessionStore((s) => s.sessions);

  if (!sessionId) return null;
  return sessions.find((s) => s.session_id === sessionId) ?? null;
}
