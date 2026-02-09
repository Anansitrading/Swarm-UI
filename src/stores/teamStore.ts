import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { TeamInfo } from "../types/team";

interface TeamState {
    teams: TeamInfo[];
    selectedTeamName: string | null;
    loading: boolean;
    enabled: boolean | null; // null = not checked yet
    error: string | null;

    checkEnabled: () => Promise<void>;
    fetchTeams: () => Promise<void>;
    initTeams: () => Promise<void>;
    createTeam: (name: string, description?: string) => Promise<void>;
    selectTeam: (name: string | null) => void;
    startWatcher: () => Promise<void>;
}

export const useTeamStore = create<TeamState>((set) => ({
    teams: [],
    selectedTeamName: null,
    loading: false,
    enabled: null,
    error: null,

    checkEnabled: async () => {
        try {
            const enabled = await invoke<boolean>("check_teams_enabled");
            set({ enabled });
        } catch {
            set({ enabled: false });
        }
    },

    fetchTeams: async () => {
        set({ loading: true, error: null });
        try {
            const teams = await invoke<TeamInfo[]>("list_teams");
            set({ teams, loading: false });
        } catch (e) {
            set({ error: String(e), loading: false });
        }
    },

    initTeams: async () => {
        set({ loading: true, error: null });
        try {
            await invoke("init_teams");
            const teams = await invoke<TeamInfo[]>("list_teams");
            set({ teams, loading: false, enabled: true });
        } catch (e) {
            set({ error: String(e), loading: false });
        }
    },

    createTeam: async (name: string, description?: string) => {
        set({ error: null });
        try {
            const team = await invoke<TeamInfo>("create_team", { name, description });
            set((state) => ({ teams: [team, ...state.teams] }));
        } catch (e) {
            set({ error: String(e) });
            throw e;
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
