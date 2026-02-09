import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { TeamInfo } from "../types/team";

interface TeamState {
    teams: TeamInfo[];
    selectedTeamName: string | null;
    loading: boolean;
    error: string | null;

    fetchTeams: () => Promise<void>;
    selectTeam: (name: string | null) => void;
    startWatcher: () => Promise<void>;
}

export const useTeamStore = create<TeamState>((set) => ({
    teams: [],
    selectedTeamName: null,
    loading: false,
    error: null,

    fetchTeams: async () => {
        set({ loading: true, error: null });
        try {
            const teams = await invoke<TeamInfo[]>("list_teams");
            set({ teams, loading: false });
        } catch (e) {
            set({ error: String(e), loading: false });
        }
    },

    selectTeam: (name) => {
        set({ selectedTeamName: name });
    },

    startWatcher: async () => {
        // Guard: only start once
        if ((useTeamStore as any)._watcherStarted) return;
        (useTeamStore as any)._watcherStarted = true;
        try {
            await invoke("start_team_watcher");

            await listen<TeamInfo>("team:updated", (event) => {
                const updated = event.payload;
                set((state) => {
                    const idx = state.teams.findIndex(
                        (t) => t.name === updated.name,
                    );
                    if (idx >= 0) {
                        const teams = [...state.teams];
                        teams[idx] = updated;
                        return { teams };
                    } else {
                        return { teams: [updated, ...state.teams] };
                    }
                });
            });
        } catch (e) {
            (useTeamStore as any)._watcherStarted = false;
            console.error("Failed to start team watcher:", e);
        }
    },
}));
