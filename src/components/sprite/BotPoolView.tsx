import { useEffect } from "react";
import { useSpriteStore } from "../../stores/spriteStore";
import { BotPoolCard } from "./BotPoolCard";

interface BotPoolViewProps {
  onSlotClick?: (spriteName: string) => void;
}

export function BotPoolView({ onSlotClick }: BotPoolViewProps) {
  const { poolState, fetchPoolState, startPoolWatcher } = useSpriteStore();

  useEffect(() => {
    fetchPoolState();
    startPoolWatcher();
  }, [fetchPoolState, startPoolWatcher]);

  if (!poolState || poolState.slots.length === 0) {
    return (
      <div className="p-4 text-center text-swarm-text-dim text-sm">
        No bot pool data. Check ~/.cortex/sprite-pool.json
      </div>
    );
  }

  return (
    <div className="p-2">
      <div className="flex items-center justify-between mb-3 px-1">
        <span className="text-xs font-semibold text-swarm-text-dim uppercase tracking-wider">
          Bot Pool
        </span>
        <div className="flex gap-3 text-xs text-swarm-text-dim">
          <span className="text-green-400">{poolState.active} active</span>
          <span>{poolState.idle} idle</span>
          <span>{poolState.total} total</span>
        </div>
      </div>
      <div className="grid grid-cols-4 gap-1.5">
        {poolState.slots.map((slot) => (
          <BotPoolCard
            key={slot.slot}
            slot={slot}
            onClick={() => {
              if (slot.sprite_name && onSlotClick) {
                onSlotClick(slot.sprite_name);
              }
            }}
          />
        ))}
      </div>
    </div>
  );
}
