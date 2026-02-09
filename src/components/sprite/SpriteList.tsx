import { useEffect } from "react";
import { useSpriteStore } from "../../stores/spriteStore";
import { useLayoutStore } from "../../stores/layoutStore";

interface SpriteListProps {
    onSelect: (name: string) => void;
}

export function SpriteList({ onSelect }: SpriteListProps) {
    const { sprites, loading, error, fetchSprites, clearError } =
        useSpriteStore();
    const { setSidebarTab } = useLayoutStore();

    useEffect(() => {
        fetchSprites();
    }, [fetchSprites]);

    if (loading && sprites.length === 0) {
        return (
            <div className="flex flex-col items-center justify-center p-8 text-swarm-text-dim text-sm gap-2">
                <div className="animate-spin h-5 w-5 border-2 border-swarm-accent border-t-transparent rounded-full" />
                <span>Loading sprites...</span>
            </div>
        );
    }

    if (error) {
        const isNotConfigured = error.includes("not configured");
        return (
            <div className="p-4 space-y-3">
                <div className="text-xs text-red-400 bg-red-400/10 border border-red-400/20 rounded p-3">
                    {isNotConfigured
                        ? "Sprites API not configured."
                        : `Error: ${error}`}
                </div>
                {isNotConfigured ? (
                    <button
                        onClick={() => setSidebarTab("settings")}
                        className="w-full px-3 py-2 text-xs bg-swarm-accent/20 text-swarm-accent border border-swarm-accent/30 rounded hover:bg-swarm-accent/30 transition-colors"
                    >
                        Configure API in Settings
                    </button>
                ) : (
                    <button
                        onClick={() => {
                            clearError();
                            fetchSprites();
                        }}
                        className="w-full px-3 py-2 text-xs bg-swarm-accent/20 text-swarm-accent border border-swarm-accent/30 rounded hover:bg-swarm-accent/30 transition-colors"
                    >
                        Retry
                    </button>
                )}
            </div>
        );
    }

    if (sprites.length === 0) {
        return (
            <div className="p-4 text-center text-swarm-text-dim text-sm">
                No sprites found. Create one from the Sprites.dev dashboard.
            </div>
        );
    }

    return (
        <div className="space-y-1 p-2">
            <div className="flex items-center justify-between px-1 mb-2">
                <span className="text-[10px] text-swarm-text-dim uppercase tracking-wide">
                    {sprites.length} sprite{sprites.length !== 1 ? "s" : ""}
                </span>
                <button
                    onClick={() => fetchSprites()}
                    className="text-[10px] text-swarm-text-dim hover:text-swarm-text transition-colors"
                >
                    Refresh
                </button>
            </div>
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
                    {sprite.region && (
                        <div className="text-[10px] text-swarm-text-dim mt-1">
                            {sprite.region}
                        </div>
                    )}
                </button>
            ))}
        </div>
    );
}
