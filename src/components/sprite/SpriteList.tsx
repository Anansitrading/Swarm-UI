import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, useCallback } from "react";
import { useSpriteStore } from "../../stores/spriteStore";
import { useLayoutStore } from "../../stores/layoutStore";
import type {
    SpriteInfo,
    SpriteSessionInfo,
    SpriteClaudeSessionInfo,
    SpriteTeamInfo,
} from "../../types/sprite";

interface SpriteListProps {
    onSelect: (name: string) => void;
}

export function SpriteList({ onSelect }: SpriteListProps) {
    const { sprites, loading, error, fetchSprites, clearError } =
        useSpriteStore();
    const { setSidebarTab } = useLayoutStore();
    const [expandedSprite, setExpandedSprite] = useState<string | null>(null);

    useEffect(() => {
        fetchSprites();
    }, [fetchSprites]);

    const handleToggleExpand = useCallback((name: string) => {
        setExpandedSprite((prev) => (prev === name ? null : name));
    }, []);

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
                <SpriteCard
                    key={sprite.name}
                    sprite={sprite}
                    expanded={expandedSprite === sprite.name}
                    onToggle={() => handleToggleExpand(sprite.name)}
                    onTerminal={() => onSelect(sprite.name)}
                />
            ))}
        </div>
    );
}

// --- SpriteCard: expandable card showing sprite details ---

interface SpriteCardProps {
    sprite: SpriteInfo;
    expanded: boolean;
    onToggle: () => void;
    onTerminal: () => void;
}

function SpriteCard({
    sprite,
    expanded,
    onToggle,
    onTerminal,
}: SpriteCardProps) {
    return (
        <div className="rounded-lg border border-swarm-border bg-swarm-surface overflow-hidden">
            {/* Header row - click to expand */}
            <button
                onClick={onToggle}
                className="w-full text-left p-2.5 hover:bg-swarm-accent/5 transition-colors"
            >
                <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                        <span className="text-[10px] text-swarm-text-dim">
                            {expanded ? "▼" : "▶"}
                        </span>
                        <span className="text-sm font-medium text-swarm-text font-mono">
                            {sprite.name}
                        </span>
                    </div>
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
                    <div className="text-[10px] text-swarm-text-dim mt-1 pl-5">
                        {sprite.region}
                    </div>
                )}
            </button>

            {/* Expanded content */}
            {expanded && (
                <SpriteExpanded
                    spriteName={sprite.name}
                    spriteStatus={sprite.status}
                    onTerminal={onTerminal}
                />
            )}
        </div>
    );
}

// --- SpriteExpanded: shows sessions/teams inside a sprite ---

interface SpriteExpandedProps {
    spriteName: string;
    spriteStatus: string;
    onTerminal: () => void;
}

function SpriteExpanded({
    spriteName,
    spriteStatus,
    onTerminal,
}: SpriteExpandedProps) {
    const [processes, setProcesses] = useState<SpriteSessionInfo[]>([]);
    const [claudeSessions, setClaudeSessions] = useState<
        SpriteClaudeSessionInfo[]
    >([]);
    const [teams, setTeams] = useState<SpriteTeamInfo[]>([]);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        if (spriteStatus !== "running") {
            setLoading(false);
            return;
        }

        let cancelled = false;
        (async () => {
            try {
                const [procs, sessions, teamList] = await Promise.allSettled([
                    invoke<SpriteSessionInfo[]>("sprite_list_sessions", {
                        name: spriteName,
                    }),
                    invoke<SpriteClaudeSessionInfo[]>(
                        "sprite_list_claude_sessions",
                        { name: spriteName },
                    ),
                    invoke<SpriteTeamInfo[]>("sprite_list_teams", {
                        name: spriteName,
                    }),
                ]);
                if (cancelled) return;
                if (procs.status === "fulfilled") setProcesses(procs.value);
                if (sessions.status === "fulfilled")
                    setClaudeSessions(sessions.value);
                if (teamList.status === "fulfilled") setTeams(teamList.value);
            } catch {
                // Silently handle errors - sprite may not be accessible
            }
            if (!cancelled) setLoading(false);
        })();

        return () => {
            cancelled = true;
        };
    }, [spriteName, spriteStatus]);

    if (spriteStatus !== "running") {
        return (
            <div className="px-3 pb-3 border-t border-swarm-border/50">
                <div className="text-[10px] text-swarm-text-dim py-2 text-center">
                    Sprite is not running
                </div>
            </div>
        );
    }

    if (loading) {
        return (
            <div className="px-3 pb-3 border-t border-swarm-border/50">
                <div className="flex items-center justify-center gap-2 py-3">
                    <div className="animate-spin h-3 w-3 border border-swarm-accent border-t-transparent rounded-full" />
                    <span className="text-[10px] text-swarm-text-dim">
                        Loading...
                    </span>
                </div>
            </div>
        );
    }

    return (
        <div className="border-t border-swarm-border/50">
            {/* Processes section */}
            <div className="px-3 py-2">
                <div className="text-[10px] text-swarm-text-dim uppercase tracking-wide mb-1">
                    Processes ({processes.length})
                </div>
                {processes.length === 0 ? (
                    <div className="text-[10px] text-swarm-text-dim/50 pl-2">
                        No relevant processes
                    </div>
                ) : (
                    <div className="space-y-0.5">
                        {processes.map((proc) => (
                            <div
                                key={proc.pid}
                                className="flex items-center gap-2 text-[10px] pl-2"
                            >
                                <span className="text-swarm-text-dim font-mono w-12 shrink-0">
                                    {proc.pid}
                                </span>
                                <span className="text-swarm-text font-mono truncate">
                                    {proc.command.slice(0, 60)}
                                </span>
                            </div>
                        ))}
                    </div>
                )}
            </div>

            {/* Claude sessions section */}
            <div className="px-3 py-2 border-t border-swarm-border/30">
                <div className="text-[10px] text-swarm-text-dim uppercase tracking-wide mb-1">
                    Claude Sessions ({claudeSessions.length})
                </div>
                {claudeSessions.length === 0 ? (
                    <div className="text-[10px] text-swarm-text-dim/50 pl-2">
                        No sessions found
                    </div>
                ) : (
                    <div className="space-y-1">
                        {claudeSessions.map((s) => (
                            <div
                                key={s.session_id}
                                className="flex items-center gap-2 text-[10px] pl-2"
                            >
                                <span className="w-1.5 h-1.5 rounded-full bg-blue-400 shrink-0" />
                                <span className="text-swarm-text font-mono truncate">
                                    {s.project_dir}
                                </span>
                                <span className="text-swarm-text-dim font-mono">
                                    {s.session_id.slice(0, 8)}
                                </span>
                            </div>
                        ))}
                    </div>
                )}
            </div>

            {/* Teams section */}
            <div className="px-3 py-2 border-t border-swarm-border/30">
                <div className="text-[10px] text-swarm-text-dim uppercase tracking-wide mb-1">
                    Teams ({teams.length})
                </div>
                {teams.length === 0 ? (
                    <div className="text-[10px] text-swarm-text-dim/50 pl-2">
                        No teams on this sprite
                    </div>
                ) : (
                    <div className="space-y-0.5">
                        {teams.map((t) => (
                            <div
                                key={t.name}
                                className="flex items-center gap-2 text-[10px] pl-2"
                            >
                                <span className="text-swarm-accent font-mono">
                                    {t.name}
                                </span>
                                <span className="text-swarm-text-dim">
                                    {t.member_count} member
                                    {t.member_count !== 1 ? "s" : ""}
                                </span>
                            </div>
                        ))}
                    </div>
                )}
            </div>

            {/* Terminal button */}
            <div className="px-3 py-2 border-t border-swarm-border/30">
                <button
                    onClick={onTerminal}
                    className="w-full px-2 py-1 text-[10px] bg-swarm-accent/10 text-swarm-accent border border-swarm-accent/20 rounded hover:bg-swarm-accent/20 transition-colors"
                >
                    Open Terminal
                </button>
            </div>
        </div>
    );
}
