import { useEffect, useCallback } from "react";
import { useLayoutStore } from "../stores/layoutStore";
import type { LayoutMode } from "../types/terminal";

/**
 * Hook for keyboard shortcuts to switch layout modes.
 * Ctrl+1: single, Ctrl+2: list, Ctrl+3: two_column, etc.
 */
export function useLayoutShortcuts() {
  const setMode = useLayoutStore((s) => s.setMode);
  const toggleSidebar = useLayoutStore((s) => s.toggleSidebar);

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
    },
    [setMode, toggleSidebar]
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);
}
