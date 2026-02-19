import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Toolbar } from "./Toolbar";
import { Sidebar } from "./Sidebar";
import { PaneGrid } from "./PaneGrid";
import { useLayoutStore } from "../../stores/layoutStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useTerminalStore } from "../../stores/terminalStore";
import { useSessionStore } from "../../stores/sessionStore";
import { useSpriteStore } from "../../stores/spriteStore";

export function AppShell() {
    const { addPane, updatePane } = useLayoutStore();
    const { loadSettings } = useSettingsStore();
    const { spawnSpriteTerminal, writeToTerminal } = useTerminalStore();
    const { selectSession } = useSessionStore();
    const [spriteError, setSpriteError] = useState<string | null>(null);

    // Load settings on app startup
    useEffect(() => {
        loadSettings();
    }, [loadSettings]);

    // Auto-dismiss sprite error after 8s
    useEffect(() => {
        if (!spriteError) return;
        const t = setTimeout(() => setSpriteError(null), 8000);
        return () => clearTimeout(t);
    }, [spriteError]);

    const handleSpriteSelect = useCallback(
        async (spriteName: string) => {
            setSpriteError(`Connecting to ${spriteName}...`);

            // Always read latest sprites from store (avoids stale closure)
            const { sprites, fetchSprites } = useSpriteStore.getState();
            const sprite = sprites.find((s) => s.name === spriteName);
            const isReachable =
                sprite &&
                (sprite.status === "warm" || sprite.status === "running");

            if (!isReachable) {
                setSpriteError(`Waking ${spriteName}...`);
                try {
                    await invoke("sprite_exec_command", {
                        name: spriteName,
                        command: "true",
                    }).catch(() => {});

                    for (let i = 0; i < 10; i++) {
                        await new Promise((r) => setTimeout(r, 2000));
                        await fetchSprites();
                        const current = useSpriteStore
                            .getState()
                            .sprites.find((s) => s.name === spriteName);
                        if (
                            current &&
                            (current.status === "warm" ||
                                current.status === "running")
                        ) {
                            break;
                        }
                    }
                } catch {
                    // Continue — WS connect will give a clear error
                }
                setSpriteError(`Connecting to ${spriteName}...`);
            }

            try {
                // 1. Push Claude credentials to the sprite
                await invoke("sprite_provision_claude", {
                    name: spriteName,
                });

                // 2. Open WebSocket terminal (bash shell)
                const info = await spawnSpriteTerminal(spriteName);

                // 3. Deselect any session so PaneGrid shows the terminal
                selectSession(null);

                // 4. Set terminal pane (update first pane if exists, else add)
                const { panes } = useLayoutStore.getState();
                if (panes.length > 0) {
                    updatePane(0, {
                        type: "terminal",
                        terminalId: info.id,
                        spriteName,
                    });
                } else {
                    addPane({
                        id: `sprite-${spriteName}-${Date.now()}`,
                        type: "terminal",
                        terminalId: info.id,
                        spriteName,
                    });
                }

                // 5. Launch Claude Code after bash initializes
                setTimeout(() => {
                    writeToTerminal(
                        info.id,
                        btoa(
                            "claude --dangerously-skip-permissions --chrome\n",
                        ),
                    );
                }, 500);
                setSpriteError(null);
            } catch (e) {
                console.error("Failed to connect to sprite:", e);
                setSpriteError(
                    `Terminal failed on ${spriteName}: ${e}`,
                );
            }
        },
        [addPane, updatePane, spawnSpriteTerminal, writeToTerminal, selectSession],
    );

    return (
        <div className="flex flex-col h-screen bg-swarm-bg">
            <Toolbar />
            <div className="flex flex-1 min-h-0 relative">
                <Sidebar onSpriteSelect={handleSpriteSelect} />
                <PaneGrid />
                {spriteError && (
                    <div className="absolute top-2 left-1/2 -translate-x-1/2 z-50 px-4 py-2 bg-red-500/90 text-white text-xs rounded-lg shadow-lg max-w-md truncate">
                        {spriteError}
                    </div>
                )}
            </div>
        </div>
    );
}
