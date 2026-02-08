import { useCallback } from "react";
import { Toolbar } from "./Toolbar";
import { Sidebar } from "./Sidebar";
import { PaneGrid } from "./PaneGrid";
import { useLayoutStore } from "../../stores/layoutStore";

export function AppShell() {
  const { addPane } = useLayoutStore();

  const handleSpriteSelect = useCallback(
    (spriteName: string) => {
      // When a sprite is selected, open a sprite terminal pane
      addPane({
        id: `sprite-${spriteName}`,
        type: "sprite",
        spriteName,
      });
    },
    [addPane]
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
