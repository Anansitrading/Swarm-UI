import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SpriteInfo, PoolState } from "../types/sprite";

interface SpriteState {
    sprites: SpriteInfo[];
    poolState: PoolState | null;
    loading: boolean;
    error: string | null;

    fetchSprites: () => Promise<void>;
    fetchPoolState: () => Promise<void>;
    startPoolWatcher: () => Promise<void>;
    execOnSprite: (name: string, command: string) => Promise<string>;
    createCheckpoint: (name: string, description: string) => Promise<string>;
    deleteSprite: (name: string) => Promise<void>;
    createSprite: (name: string) => Promise<void>;
    clearError: () => void;
}

export const useSpriteStore = create<SpriteState>((set) => ({
    sprites: [],
    poolState: null,
    loading: false,
    error: null,

    fetchSprites: async () => {
        set({ loading: true, error: null });
        try {
            const sprites = await invoke<SpriteInfo[]>("sprite_list");
            set({ sprites, loading: false });
        } catch (e) {
            set({
                loading: false,
                error: String(e),
            });
        }
    },

    fetchPoolState: async () => {
        try {
            const poolState = await invoke<PoolState>("get_bot_pool_state");
            set({ poolState });
        } catch {
            // Pool file may not exist
        }
    },

    startPoolWatcher: async () => {
        try {
            await invoke("start_pool_watcher");
            await listen<PoolState>("pool:updated", (event) => {
                set({ poolState: event.payload });
            });
        } catch (e) {
            console.error("Failed to start pool watcher:", e);
        }
    },

    execOnSprite: async (name: string, command: string) => {
        return await invoke<string>("sprite_exec", { name, command });
    },

    createCheckpoint: async (name: string, description: string) => {
        return await invoke<string>("sprite_checkpoint_create", {
            name,
            description,
        });
    },

    deleteSprite: async (name: string) => {
        await invoke("sprite_delete", { name });
        set((state) => ({
            sprites: state.sprites.filter((s) => s.name !== name),
        }));
    },

    createSprite: async (name: string) => {
        const sprite = await invoke<SpriteInfo>("sprite_create", { name });
        set((state) => ({
            sprites: [...state.sprites, sprite],
        }));
    },

    clearError: () => set({ error: null }),
}));
