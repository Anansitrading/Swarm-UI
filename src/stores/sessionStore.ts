import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SessionListItem, IndexProgress } from "../types/session";

interface SessionState {
    sessions: SessionListItem[];
    selectedSessionId: string | null;
    loading: boolean;
    error: string | null;
    searchQuery: string;
    indexProgress: IndexProgress | null;

    fetchSessions: () => Promise<void>;
    selectSession: (id: string | null) => void;
    setSearchQuery: (q: string) => void;
    listenForUpdates: () => Promise<void>;
}

export const useSessionStore = create<SessionState>((set) => ({
    sessions: [],
    selectedSessionId: null,
    loading: false,
    error: null,
    searchQuery: "",
    indexProgress: null,

    fetchSessions: async () => {
        set({ loading: true, error: null });
        try {
            const sessions = await invoke<SessionListItem[]>("list_sessions");
            set({ sessions, loading: false });
        } catch (e) {
            set({ error: String(e), loading: false });
        }
    },

    selectSession: (id) => {
        set({ selectedSessionId: id });
    },

    setSearchQuery: (q) => {
        set({ searchQuery: q });
    },

    listenForUpdates: async () => {
        // Guard: only start once
        if ((useSessionStore as any)._listenerStarted) return;
        (useSessionStore as any)._listenerStarted = true;
        try {
            // Surgical upsert on session:updated
            await listen<SessionListItem>("session:updated", (event) => {
                const updated = event.payload;
                set((state) => {
                    const idx = state.sessions.findIndex(
                        (s) => s.session_id === updated.session_id,
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

            // Index progress events
            await listen<IndexProgress>("index:progress", (event) => {
                set({ indexProgress: event.payload });
            });
        } catch (e) {
            (useSessionStore as any)._listenerStarted = false;
            console.error("Failed to start event listeners:", e);
        }
    },
}));
