import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { PtyInfo, PtySpawnConfig } from "../types/terminal";

interface TerminalState {
    terminals: PtyInfo[];
    activeTerminalId: string | null;
    // Track which terminals are sprite WebSocket sessions vs local PTY
    spriteTerminals: Set<string>;

    spawnTerminal: (config?: PtySpawnConfig) => Promise<PtyInfo>;
    spawnSpriteTerminal: (
        spriteName: string,
        cols?: number,
        rows?: number,
    ) => Promise<PtyInfo>;
    killTerminal: (id: string) => Promise<void>;
    setActiveTerminal: (id: string | null) => void;
    writeToTerminal: (id: string, data: string) => Promise<void>;
    resizeTerminal: (id: string, cols: number, rows: number) => Promise<void>;
    isSpriteTerminal: (id: string) => boolean;
}

export const useTerminalStore = create<TerminalState>((set, get) => ({
    terminals: [],
    activeTerminalId: null,
    spriteTerminals: new Set(),

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

    spawnSpriteTerminal: async (
        spriteName: string,
        cols?: number,
        rows?: number,
    ) => {
        const info = await invoke<PtyInfo>("sprite_ws_spawn", {
            spriteName,
            cols: cols ?? 80,
            rows: rows ?? 24,
        });
        set((state) => {
            const newSpriteTerminals = new Set(state.spriteTerminals);
            newSpriteTerminals.add(info.id);
            return {
                terminals: [...state.terminals, info],
                activeTerminalId: info.id,
                spriteTerminals: newSpriteTerminals,
            };
        });
        return info;
    },

    killTerminal: async (id: string) => {
        const isSprite = get().spriteTerminals.has(id);
        if (isSprite) {
            await invoke("sprite_ws_kill", { id });
        } else {
            await invoke("pty_kill", { id });
        }
        set((state) => {
            const newSpriteTerminals = new Set(state.spriteTerminals);
            newSpriteTerminals.delete(id);
            return {
                terminals: state.terminals.filter((t) => t.id !== id),
                activeTerminalId:
                    state.activeTerminalId === id
                        ? null
                        : state.activeTerminalId,
                spriteTerminals: newSpriteTerminals,
            };
        });
    },

    setActiveTerminal: (id) => {
        set({ activeTerminalId: id });
    },

    writeToTerminal: async (id: string, data: string) => {
        const isSprite = get().spriteTerminals.has(id);
        if (isSprite) {
            await invoke("sprite_ws_write", { id, data });
        } else {
            await invoke("pty_write", { id, data });
        }
    },

    resizeTerminal: async (id: string, cols: number, rows: number) => {
        const isSprite = get().spriteTerminals.has(id);
        if (isSprite) {
            await invoke("sprite_ws_resize", { id, cols, rows });
        } else {
            await invoke("pty_resize", { id, cols, rows });
        }
    },

    isSpriteTerminal: (id: string) => {
        return get().spriteTerminals.has(id);
    },
}));
