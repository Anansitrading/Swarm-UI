import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { SpriteInfo } from "../types/sprite";

interface SpriteState {
    sprites: SpriteInfo[];
    loading: boolean;
    error: string | null;

    fetchSprites: () => Promise<void>;
    execOnSprite: (name: string, command: string) => Promise<string>;
    createCheckpoint: (name: string, description: string) => Promise<string>;
    deleteSprite: (name: string) => Promise<void>;
    createSprite: (name: string) => Promise<void>;
    clearError: () => void;
}

export const useSpriteStore = create<SpriteState>((set) => ({
    sprites: [],
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
