import { useEffect, useCallback } from "react";
import { useLayoutStore } from "../stores/layoutStore";
import { useTerminalStore } from "../stores/terminalStore";
import type { LayoutMode } from "../types/terminal";

/**
 * Hook for keyboard shortcuts.
 * Ctrl+1-5: layout modes, Ctrl+B: toggle sidebar, Ctrl+T: new terminal
 */
export function useLayoutShortcuts() {
  const setMode = useLayoutStore((s) => s.setMode);
  const toggleSidebar = useLayoutStore((s) => s.toggleSidebar);
  const addPane = useLayoutStore((s) => s.addPane);
  const { spawnTerminal } = useTerminalStore();

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (!e.ctrlKey) return;

      const modeMap: Record<string, LayoutMode> = {
        "1": "single",
        "2": "list",
        "3": "two_column",
        "4": "three_column",
        "5": "sprite_grid",
      };

      if (e.key in modeMap) {
        e.preventDefault();
        setMode(modeMap[e.key]);
      }

      if (e.key === "b") {
        e.preventDefault();
        toggleSidebar();
      }

      // Ctrl+T: new terminal pane
      if (e.key === "t") {
        e.preventDefault();
        spawnTerminal().then((info) => {
          addPane({ id: `terminal-${info.id}`, type: "terminal", terminalId: info.id });
        });
      }
    },
    [setMode, toggleSidebar, addPane, spawnTerminal]
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);
}
