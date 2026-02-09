import { useState, useCallback } from "react";
import { useLayoutStore } from "../../stores/layoutStore";
import { useTerminalStore } from "../../stores/terminalStore";
import { AgentPicker } from "../terminal/AgentPicker";
import type { LayoutMode } from "../../types/terminal";

const LAYOUT_OPTIONS: {
    mode: LayoutMode;
    icon: string;
    label: string;
    shortcut: string;
}[] = [
    { mode: "single", icon: "□", label: "Single", shortcut: "Ctrl+1" },
    { mode: "list", icon: "▌□", label: "List", shortcut: "Ctrl+2" },
    {
        mode: "two_column",
        icon: "□□",
        label: "Two Columns",
        shortcut: "Ctrl+3",
    },
    {
        mode: "three_column",
        icon: "□□□",
        label: "Three Columns",
        shortcut: "Ctrl+4",
    },
    {
        mode: "sprite_grid",
        icon: "⊞",
        label: "Sprite Grid",
        shortcut: "Ctrl+5",
    },
];

/** Build the shell command to launch an agent session */
function agentCommand(agentName: string): { shell: string; args: string[] } {
    if (agentName === "claude") {
        return {
            shell: "claude",
            args: ["--dangerously-skip-permissions", "--chrome"],
        };
    }
    // Custom agents use claude with --agent
    return {
        shell: "claude",
        args: [
            "--dangerously-skip-permissions",
            "--chrome",
            "--agent",
            agentName,
        ],
    };
}

export function Toolbar() {
    const { mode, setMode, toggleSidebar, sidebarCollapsed, addPane } =
        useLayoutStore();
    const { spawnTerminal } = useTerminalStore();
    const [showAgentPicker, setShowAgentPicker] = useState(false);

    const handleSpawnAgent = useCallback(
        async (agentName: string) => {
            setShowAgentPicker(false);
            try {
                const cmd = agentCommand(agentName);
                const info = await spawnTerminal({
                    shell: cmd.shell,
                    args: cmd.args,
                });
                addPane({
                    id: `terminal-${info.id}`,
                    type: "terminal",
                    terminalId: info.id,
                });
            } catch (e) {
                console.error("Failed to spawn agent terminal:", e);
            }
        },
        [spawnTerminal, addPane],
    );

    return (
        <div className="flex items-center justify-between h-10 px-3 bg-swarm-surface border-b border-swarm-border select-none">
            <div className="flex items-center gap-2">
                <button
                    onClick={toggleSidebar}
                    className="text-swarm-text-dim hover:text-swarm-text p-1 rounded transition-colors"
                    title={`${sidebarCollapsed ? "Show" : "Hide"} sidebar (Ctrl+B)`}
                >
                    <svg
                        className="w-4 h-4"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        strokeWidth={2}
                    >
                        <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5"
                        />
                    </svg>
                </button>
                <span className="text-sm font-semibold text-swarm-accent">
                    Swarm-UI
                </span>
            </div>

            <div className="flex items-center gap-1 bg-swarm-bg rounded-lg p-0.5">
                {LAYOUT_OPTIONS.map((opt) => (
                    <button
                        key={opt.mode}
                        onClick={() => setMode(opt.mode)}
                        className={`px-2 py-1 rounded text-xs font-mono transition-colors ${
                            mode === opt.mode
                                ? "bg-swarm-accent text-white"
                                : "text-swarm-text-dim hover:text-swarm-text"
                        }`}
                        title={`${opt.label} (${opt.shortcut})`}
                    >
                        {opt.icon}
                    </button>
                ))}
            </div>

            <div className="relative flex items-center gap-1">
                <button
                    onClick={() => setShowAgentPicker((v) => !v)}
                    className="text-swarm-text-dim hover:text-swarm-text px-2 py-1 rounded text-xs transition-colors hover:bg-swarm-accent/10"
                    title="New Agent Session (Ctrl+T)"
                >
                    + Terminal
                </button>
                {showAgentPicker && (
                    <AgentPicker
                        onSelect={handleSpawnAgent}
                        onClose={() => setShowAgentPicker(false)}
                        position="toolbar"
                    />
                )}
            </div>
        </div>
    );
}
