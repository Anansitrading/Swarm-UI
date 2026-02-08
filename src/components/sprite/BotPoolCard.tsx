import type { BotSlot } from "../../types/sprite";

interface BotPoolCardProps {
  slot: BotSlot;
  onClick: () => void;
}

export function BotPoolCard({ slot, onClick }: BotPoolCardProps) {
  const isActive = slot.status === "active" || slot.status === "claimed";
  const hasSprite = !!slot.sprite_name;

  return (
    <button
      onClick={onClick}
      disabled={!hasSprite}
      className={`p-1.5 rounded text-center border transition-colors ${
        isActive
          ? "border-green-500/30 bg-green-500/5 hover:bg-green-500/10"
          : hasSprite
            ? "border-swarm-border bg-swarm-surface hover:border-swarm-accent/20"
            : "border-swarm-border/50 bg-swarm-bg opacity-50 cursor-default"
      }`}
    >
      <div className="text-xs font-mono font-bold text-swarm-text">
        {slot.slot}
      </div>
      {slot.role && (
        <div className="text-[10px] text-swarm-text-dim truncate mt-0.5">
          {slot.role}
        </div>
      )}
      {isActive && (
        <div className="w-1.5 h-1.5 rounded-full bg-green-400 mx-auto mt-1" />
      )}
    </button>
  );
}
