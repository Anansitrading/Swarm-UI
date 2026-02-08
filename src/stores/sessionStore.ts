import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SessionInfo } from "../types/session";

interface SessionState {
  sessions: SessionInfo[];
  selectedSessionId: string | null;
  loading: boolean;
  error: string | null;

  fetchSessions: () => Promise<void>;
  selectSession: (id: string | null) => void;
  startWatcher: () => Promise<void>;
}

export const useSessionStore = create<SessionState>((set) => ({
  sessions: [],
  selectedSessionId: null,
  loading: false,
  error: null,

  fetchSessions: async () => {
    set({ loading: true, error: null });
    try {
      const sessions = await invoke<SessionInfo[]>("list_sessions");
      set({ sessions, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  selectSession: (id) => {
    set({ selectedSessionId: id });
  },

  startWatcher: async () => {
    try {
      await invoke("start_session_watcher");

      // Listen for session update events from Rust backend
      await listen<SessionInfo>("session:updated", (event) => {
        const updated = event.payload;
        set((state) => {
          const idx = state.sessions.findIndex(
            (s) => s.jsonl_path === updated.jsonl_path
          );
          if (idx >= 0) {
            const sessions = [...state.sessions];
            sessions[idx] = updated;
            return { sessions };
          } else {
            return { sessions: [updated, ...state.sessions] };
          }
        });
      });
    } catch (e) {
      console.error("Failed to start session watcher:", e);
    }
  },
}));
