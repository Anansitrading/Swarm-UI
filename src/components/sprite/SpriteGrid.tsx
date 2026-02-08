import { useSpriteStore } from "../../stores/spriteStore";
import { SpriteTerminal } from "../terminal/SpriteTerminal";

export function SpriteGrid() {
  const { sprites } = useSpriteStore();
  const activeSprites = sprites.filter((s) => s.status === "running");

  if (activeSprites.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-swarm-text-dim">
        No active sprites. Start sprites to see them here.
      </div>
    );
  }

  return (
    <div
      className="grid gap-1 h-full w-full p-1"
      style={{
        gridTemplateColumns: "repeat(auto-fill, minmax(400px, 1fr))",
        gridAutoRows: "minmax(250px, 1fr)",
      }}
    >
      {activeSprites.map((sprite) => (
        <div
          key={sprite.name}
          className="rounded border border-swarm-border overflow-hidden flex flex-col"
        >
          <div className="flex items-center justify-between px-2 py-1 bg-swarm-surface border-b border-swarm-border">
            <span className="text-xs font-mono text-swarm-text">
              {sprite.name}
            </span>
            <span className="text-[10px] text-green-400">running</span>
          </div>
          <div className="flex-1 min-h-0">
            <SpriteTerminal spriteName={sprite.name} className="h-full" />
          </div>
        </div>
      ))}
    </div>
  );
}
