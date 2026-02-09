import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface Settings {
  spriteApiUrl: string;
  spriteApiToken: string;
  spriteOrg: string;
  terminalFont: string;
  terminalFontSize: number;
}

const DEFAULT_SETTINGS: Settings = {
  spriteApiUrl: "https://api.sprites.dev",
  spriteApiToken: "",
  spriteOrg: "david-simpson",
  terminalFont: "JetBrains Mono",
  terminalFontSize: 13,
};

interface SettingsState {
  settings: Settings;
  loaded: boolean;
  connectionStatus: string | null;
  testing: boolean;

  loadSettings: () => Promise<void>;
  saveSettings: (settings: Partial<Settings>) => Promise<void>;
  testConnection: () => Promise<void>;
  configureSpriteApi: () => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  loaded: false,
  connectionStatus: null,
  testing: false,

  loadSettings: async () => {
    try {
      // Try to load from localStorage (simple persistence for Tauri webview)
      const stored = localStorage.getItem("swarm-ui-settings");
      if (stored) {
        const parsed = JSON.parse(stored) as Partial<Settings>;
        const settings = { ...DEFAULT_SETTINGS, ...parsed };
        set({ settings, loaded: true });

        // Auto-configure sprites API if token exists
        if (settings.spriteApiToken) {
          try {
            await invoke("sprite_configure", {
              baseUrl: settings.spriteApiUrl,
              token: settings.spriteApiToken,
            });
          } catch {
            // Settings loaded but API not reachable yet - that's ok
          }
        }
      } else {
        set({ loaded: true });
      }
    } catch {
      set({ loaded: true });
    }
  },

  saveSettings: async (partial: Partial<Settings>) => {
    const current = get().settings;
    const settings = { ...current, ...partial };
    set({ settings });

    // Persist to localStorage
    localStorage.setItem("swarm-ui-settings", JSON.stringify(settings));

    // If sprite settings changed, reconfigure the API client
    if (partial.spriteApiUrl || partial.spriteApiToken) {
      if (settings.spriteApiToken) {
        try {
          await invoke("sprite_configure", {
            baseUrl: settings.spriteApiUrl,
            token: settings.spriteApiToken,
          });
        } catch {
          // Will show error on explicit test
        }
      }
    }
  },

  testConnection: async () => {
    set({ testing: true, connectionStatus: null });
    try {
      const result = await invoke<string>("sprite_test_connection");
      set({ connectionStatus: result, testing: false });
    } catch (e) {
      set({
        connectionStatus: `Error: ${e}`,
        testing: false,
      });
    }
  },

  configureSpriteApi: async () => {
    const { settings } = get();
    if (!settings.spriteApiToken) {
      set({ connectionStatus: "No API token configured" });
      return;
    }
    try {
      const result = await invoke<string>("sprite_configure", {
        baseUrl: settings.spriteApiUrl,
        token: settings.spriteApiToken,
      });
      set({ connectionStatus: result });
    } catch (e) {
      set({ connectionStatus: `Error: ${e}` });
    }
  },
}));
