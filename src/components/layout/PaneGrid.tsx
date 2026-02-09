import { useCallback } from "react";
import { useLayoutStore } from "../../stores/layoutStore";
import { useSessionStore } from "../../stores/sessionStore";
import { useTerminalStore } from "../../stores/terminalStore";
import { TerminalPane } from "../terminal/TerminalPane";
import { SessionDetail } from "../session/SessionDetail";
import { SpriteGrid } from "../sprite/SpriteGrid";
import { FileChangeList } from "../diff/FileChangeList";
import type { LayoutMode } from "../../types/terminal";
import { invoke } from "@tauri-apps/api/core";

function gridStyle(mode: LayoutMode): React.CSSProperties {
  switch (mode) {
    case "single":
      return { gridTemplateColumns: "1fr" };
    case "list":
      return { gridTemplateColumns: "1fr" };
    case "two_column":
      return { gridTemplateColumns: "1fr 1fr" };
    case "three_column":
      return { gridTemplateColumns: "1fr 1fr 1fr" };
    case "sprite_grid":
      return {}; // SpriteGrid handles its own layout
  }
}

export function PaneGrid() {
  const { mode, panes, addPane } = useLayoutStore();
  const { selectedSessionId, sessions } = useSessionStore();
  const { spawnTerminal } = useTerminalStore();

  const selectedSession = sessions.find((s) => s.id === selectedSessionId);

  const handleOpenTerminal = useCallback(
    async (cwd: string) => {
      const info = await spawnTerminal({ cwd });
      addPane({ id: `terminal-${info.id}`, type: "terminal", terminalId: info.id });
    },
    [spawnTerminal, addPane]
  );

  const handleKillProcess = useCallback(async (pid: number) => {
    try {
      await invoke("kill_process", { pid, force: false });
    } catch (e) {
      console.error("Failed to kill process:", e);
    }
  }, []);

  if (mode === "sprite_grid") {
    return (
      <div className="flex-1 min-h-0 overflow-hidden">
        <SpriteGrid />
      </div>
    );
  }

  // In "list" mode, show session detail alongside panes
  if (mode === "list" && selectedSession) {
    return (
      <div className="flex-1 min-h-0 grid grid-cols-2 gap-1 p-1 overflow-hidden">
        {/* Session detail panel */}
        <div className="min-h-0 rounded border border-swarm-border overflow-hidden">
          <SessionDetail
            session={selectedSession}
            onOpenTerminal={handleOpenTerminal}
            onKillProcess={handleKillProcess}
          />
        </div>
        {/* Terminal panes */}
        <div className="min-h-0 rounded border border-swarm-border overflow-hidden">
          {panes.length > 0 && panes[0].type === "terminal" ? (
            <TerminalPane ptyId={panes[0].terminalId} />
          ) : (
            <TerminalPane />
          )}
        </div>
      </div>
    );
  }

  return (
    <div
      className="flex-1 min-h-0 grid gap-1 p-1 overflow-hidden"
      style={gridStyle(mode)}
    >
      {panes.map((pane) => (
        <div
          key={pane.id}
          className="min-h-0 rounded border border-swarm-border overflow-hidden"
        >
          {pane.type === "terminal" && (
            <TerminalPane ptyId={pane.terminalId} />
          )}
          {pane.type === "sprite" && (
            <TerminalPane
              spawnConfig={{
                shell: "sprite",
                args: ["console", "-s", pane.spriteName || ""],
              }}
            />
          )}
          {pane.type === "diff" && (
            <FileChangeList changes={[]} />
          )}
          {pane.type === "empty" && (
            <div className="flex items-center justify-center h-full text-swarm-text-dim text-sm">
              Empty pane
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
