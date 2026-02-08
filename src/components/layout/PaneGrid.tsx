import { useLayoutStore } from "../../stores/layoutStore";
import { TerminalPane } from "../terminal/TerminalPane";
import { SpriteGrid } from "../sprite/SpriteGrid";
import type { LayoutMode } from "../../types/terminal";

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
  const { mode, panes } = useLayoutStore();

  if (mode === "sprite_grid") {
    return (
      <div className="flex-1 min-h-0 overflow-hidden">
        <SpriteGrid />
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
