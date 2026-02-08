import { useEffect } from "react";
import { useSpriteStore } from "../../stores/spriteStore";

interface SpriteListProps {
  onSelect: (name: string) => void;
}

export function SpriteList({ onSelect }: SpriteListProps) {
  const { sprites, loading, fetchSprites } = useSpriteStore();

  useEffect(() => {
    fetchSprites();
  }, [fetchSprites]);

  if (loading && sprites.length === 0) {
    return (
      <div className="flex items-center justify-center p-8 text-swarm-text-dim text-sm">
        Loading sprites...
      </div>
    );
  }

  if (sprites.length === 0) {
    return (
      <div className="p-4 text-center text-swarm-text-dim text-sm">
        No sprites found. Use <code className="text-swarm-accent">sprite create</code> to add one.
      </div>
    );
  }

  return (
    <div className="space-y-1 p-2">
      {sprites.map((sprite) => (
        <button
          key={sprite.name}
          onClick={() => onSelect(sprite.name)}
          className="w-full text-left p-2.5 rounded-lg border border-swarm-border hover:border-swarm-accent/20 bg-swarm-surface transition-colors"
        >
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium text-swarm-text font-mono">
              {sprite.name}
            </span>
            <span
              className={`text-xs px-1.5 py-0.5 rounded ${
                sprite.status === "running"
                  ? "text-green-400 bg-green-400/10"
                  : sprite.status === "stopped"
                    ? "text-red-400 bg-red-400/10"
                    : "text-gray-400 bg-gray-400/10"
              }`}
            >
              {sprite.status}
            </span>
          </div>
        </button>
      ))}
    </div>
  );
}
