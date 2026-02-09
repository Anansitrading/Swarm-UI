import { useCallback, useEffect } from "react";
import { Toolbar } from "./Toolbar";
import { Sidebar } from "./Sidebar";
import { PaneGrid } from "./PaneGrid";
import { useLayoutStore } from "../../stores/layoutStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useTerminalStore } from "../../stores/terminalStore";

export function AppShell() {
    const { addPane } = useLayoutStore();
    const { loadSettings } = useSettingsStore();
    const { spawnSpriteTerminal } = useTerminalStore();

    // Load settings on app startup
    useEffect(() => {
        loadSettings();
    }, [loadSettings]);

    const handleSpriteSelect = useCallback(
        async (spriteName: string) => {
            try {
                // Open a WebSocket terminal to the sprite
                const info = await spawnSpriteTerminal(spriteName);
                addPane({
                    id: `sprite-${info.id}`,
                    type: "terminal",
                    terminalId: info.id,
                    spriteName,
                });
            } catch (e) {
                console.error("Failed to connect to sprite:", e);
                // Still add a pane to show the error state
                addPane({
                    id: `sprite-${spriteName}`,
                    type: "sprite",
                    spriteName,
                });
            }
        },
        [addPane, spawnSpriteTerminal],
    );

    return (
        <div className="flex flex-col h-screen bg-swarm-bg">
            <Toolbar />
            <div className="flex flex-1 min-h-0">
                <Sidebar onSpriteSelect={handleSpriteSelect} />
                <PaneGrid />
            </div>
        </div>
    );
}
