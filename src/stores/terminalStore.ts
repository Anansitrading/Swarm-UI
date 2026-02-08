import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { PtyInfo, PtySpawnConfig } from "../types/terminal";

interface TerminalState {
  terminals: PtyInfo[];
  activeTerminalId: string | null;

  spawnTerminal: (config?: PtySpawnConfig) => Promise<PtyInfo>;
  killTerminal: (id: string) => Promise<void>;
  setActiveTerminal: (id: string | null) => void;
  writeToTerminal: (id: string, data: string) => Promise<void>;
  resizeTerminal: (id: string, cols: number, rows: number) => Promise<void>;
}

export const useTerminalStore = create<TerminalState>((set) => ({
  terminals: [],
  activeTerminalId: null,

  spawnTerminal: async (config?: PtySpawnConfig) => {
    const info = await invoke<PtyInfo>("pty_spawn", {
      config: config ?? {},
    });
    set((state) => ({
      terminals: [...state.terminals, info],
      activeTerminalId: info.id,
    }));
    return info;
  },

  killTerminal: async (id: string) => {
    await invoke("pty_kill", { id });
    set((state) => ({
      terminals: state.terminals.filter((t) => t.id !== id),
      activeTerminalId:
        state.activeTerminalId === id ? null : state.activeTerminalId,
    }));
  },

  setActiveTerminal: (id) => {
    set({ activeTerminalId: id });
  },

  writeToTerminal: async (id: string, data: string) => {
    await invoke("pty_write", { id, data });
  },

  resizeTerminal: async (id: string, cols: number, rows: number) => {
    await invoke("pty_resize", { id, cols, rows });
  },
}));
