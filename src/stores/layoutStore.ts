import { create } from "zustand";
import type { LayoutMode, PaneConfig } from "../types/terminal";

interface LayoutState {
    mode: LayoutMode;
    panes: PaneConfig[];
    sidebarCollapsed: boolean;
    sidebarTab: "sessions" | "teams" | "sprites" | "settings";

    setMode: (mode: LayoutMode, keepPanes?: boolean) => void;
    toggleSidebar: () => void;
    setSidebarTab: (tab: "sessions" | "teams" | "sprites" | "settings") => void;
    updatePane: (index: number, config: Partial<PaneConfig>) => void;
    addPane: (config: PaneConfig) => void;
    removePane: (id: string) => void;
}

function defaultPanes(mode: LayoutMode): PaneConfig[] {
    switch (mode) {
        case "single":
            return [{ id: "pane-0", type: "terminal" }];
        case "list":
            return [{ id: "pane-0", type: "terminal" }];
        case "two_column":
            return [
                { id: "pane-0", type: "terminal" },
                { id: "pane-1", type: "terminal" },
            ];
        case "three_column":
            return [
                { id: "pane-0", type: "terminal" },
                { id: "pane-1", type: "terminal" },
                { id: "pane-2", type: "terminal" },
            ];
        case "sprite_grid":
            return []; // Dynamically filled from sprite list
    }
}

export const useLayoutStore = create<LayoutState>((set) => ({
    mode: "list",
    panes: defaultPanes("list"),
    sidebarCollapsed: false,
    sidebarTab: "sessions",

    setMode: (mode, keepPanes) => {
        if (keepPanes) {
            set({ mode });
        } else {
            set({ mode, panes: defaultPanes(mode) });
        }
    },

    toggleSidebar: () => {
        set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed }));
    },

    setSidebarTab: (tab) => {
        set({ sidebarTab: tab });
    },

    updatePane: (index, config) => {
        set((state) => {
            const panes = [...state.panes];
            if (panes[index]) {
                panes[index] = { ...panes[index], ...config };
            }
            return { panes };
        });
    },

    addPane: (config) => {
        set((state) => ({ panes: [...state.panes, config] }));
    },

    removePane: (id) => {
        set((state) => ({
            panes: state.panes.filter((p) => p.id !== id),
        }));
    },
}));
