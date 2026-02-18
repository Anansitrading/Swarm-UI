import { useEffect, useState, useMemo } from "react";
import { useSessionStore } from "../../stores/sessionStore";
import { SessionCard } from "./SessionCard";
import { useSessionSearch } from "../../hooks/useSessionSearch";
import { HighlightText } from "./HighlightText";
import type { SessionListItem } from "../../types/session";

interface RepoGroup {
    name: string;
    path: string;
    sessions: SessionListItem[];
    expanded: boolean;
}

/** Parse ISO date string to epoch ms, fallback to 0 */
function dateToEpoch(dateStr?: string): number {
    if (!dateStr) return 0;
    const ms = Date.parse(dateStr);
    return isNaN(ms) ? 0 : ms;
}

export function SessionList() {
    const {
        sessions,
        selectedSessionId,
        loading,
        fetchSessions,
        selectSession,
    } = useSessionStore();
    const { query: search, setQuery: setSearch, matchedSessionIds, isSearching, getHighlightRanges } = useSessionSearch(sessions);
    const [expandedRepos, setExpandedRepos] = useState<Set<string>>(new Set());
    // Track repos the user has manually collapsed so auto-expand doesn't fight them
    const [manuallyCollapsed, setManuallyCollapsed] = useState<Set<string>>(new Set());

    // App.tsx calls fetchSessions() and listenForUpdates() on mount.
    // SessionList only needs to ensure sessions are loaded if they aren't yet.
    useEffect(() => {
        if (sessions.length === 0 && !loading) {
            fetchSessions();
        }
    }, []); // eslint-disable-line react-hooks/exhaustive-deps

    // Group sessions by repository (project_path)
    const baseGroups = useMemo(() => {
        const groups = new Map<string, SessionListItem[]>();

        for (const session of sessions) {
            const repoPath = session.project_path;
            const existing = groups.get(repoPath) || [];
            existing.push(session);
            groups.set(repoPath, existing);
        }

        const result: RepoGroup[] = [];
        for (const [path, groupSessions] of groups.entries()) {
            const name = path.split("/").pop() || path;
            result.push({
                name,
                path,
                sessions: groupSessions.sort(
                    (a, b) => dateToEpoch(b.modified_at) - dateToEpoch(a.modified_at),
                ),
                expanded: false, // Applied later from expandedRepos
            });
        }

        result.sort((a, b) => {
            const aMax = Math.max(...a.sessions.map((s) => dateToEpoch(s.modified_at)));
            const bMax = Math.max(...b.sessions.map((s) => dateToEpoch(s.modified_at)));
            return bMax - aMax;
        });

        return result;
    }, [sessions]);

    // Apply expansion state separately to break circular dependency
    const repoGroups = useMemo(() => {
        return baseGroups.map(g => ({
            ...g,
            expanded: expandedRepos.has(g.path),
        }));
    }, [baseGroups, expandedRepos]);

    // Filter by search
    const filteredGroups = useMemo(() => {
        if (!isSearching || !matchedSessionIds) return repoGroups;
        return repoGroups
            .map((g) => ({
                ...g,
                sessions: g.sessions.filter((s) => matchedSessionIds.has(s.session_id)),
                expanded: true, // Auto-expand all groups when searching
            }))
            .filter((g) => g.sessions.length > 0);
    }, [repoGroups, isSearching, matchedSessionIds]);

    const toggleRepo = (path: string) => {
        setExpandedRepos((prev) => {
            const next = new Set(prev);
            if (next.has(path)) {
                next.delete(path);
                setManuallyCollapsed((mc) => new Set(mc).add(path));
            } else {
                next.add(path);
                setManuallyCollapsed((mc) => {
                    const updated = new Set(mc);
                    updated.delete(path);
                    return updated;
                });
            }
            return next;
        });
    };

    // Auto-expand repos with selected session or active sessions,
    // but respect repos the user has manually collapsed.
    useEffect(() => {
        setExpandedRepos(prev => {
            const newExpanded = new Set(prev);
            let changed = false;
            for (const group of baseGroups) {
                if (manuallyCollapsed.has(group.path)) continue;
                const hasSelected = group.sessions.some(
                    (s) => s.session_id === selectedSessionId,
                );
                const hasActive = group.sessions.some(
                    (s) =>
                        s.status === "thinking" ||
                        s.status === "executing_tool",
                );
                if ((hasSelected || hasActive) && !newExpanded.has(group.path)) {
                    newExpanded.add(group.path);
                    changed = true;
                }
            }
            return changed ? newExpanded : prev;
        });
    }, [selectedSessionId, baseGroups, manuallyCollapsed]);

    const totalSessions = sessions.length;

    if (loading && sessions.length === 0) {
        return (
            <div className="flex items-center justify-center p-8 text-swarm-text-dim text-sm">
                Loading sessions...
            </div>
        );
    }

    return (
        <div className="flex flex-col h-full">
            {/* Search bar */}
            <div className="p-2 border-b border-swarm-border">
                <div className="relative">
                    <svg
                        className="absolute left-2 top-1/2 -translate-y-1/2 w-3 h-3 text-swarm-text-dim"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        strokeWidth={2}
                    >
                        <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z"
                        />
                    </svg>
                    <input
                        type="text"
                        value={search}
                        onChange={(e) => setSearch(e.target.value)}
                        placeholder="Search all sessions..."
                        className="w-full bg-swarm-bg border border-swarm-border rounded text-xs text-swarm-text pl-7 pr-2 py-1.5 placeholder-swarm-text-dim/50 focus:border-swarm-accent/50 focus:outline-none"
                    />
                </div>
                <div className="text-[10px] text-swarm-text-dim mt-1.5 flex items-center justify-between">
                    <span>
                        {filteredGroups.length} repos · {totalSessions} sessions
                    </span>
                    <button
                        onClick={() => fetchSessions()}
                        className="hover:text-swarm-text transition-colors"
                    >
                        Refresh
                    </button>
                </div>
            </div>

            {/* Repo tree */}
            <div className="flex-1 overflow-y-auto">
                {filteredGroups.length === 0 ? (
                    <div className="p-4 text-center text-swarm-text-dim text-sm">
                        {sessions.length === 0
                            ? "No active sessions found."
                            : "No matches found."}
                    </div>
                ) : (
                    filteredGroups.map((group) => (
                        <RepoSection
                            key={group.path}
                            group={group}
                            selectedSessionId={selectedSessionId}
                            onToggle={() => toggleRepo(group.path)}
                            onSelectSession={selectSession}
                            getHighlightRanges={isSearching ? getHighlightRanges : undefined}
                        />
                    ))
                )}
            </div>
        </div>
    );
}

function RepoSection({
    group,
    selectedSessionId,
    onToggle,
    onSelectSession,
    getHighlightRanges,
}: {
    group: RepoGroup;
    selectedSessionId: string | null;
    onToggle: () => void;
    onSelectSession: (id: string | null) => void;
    getHighlightRanges?: (text: string) => { start: number; end: number }[];
}) {
    const activeCount = group.sessions.filter(
        (s) =>
            s.status === "thinking" || s.status === "executing_tool",
    ).length;

    return (
        <div className="border-b border-swarm-border/50">
            {/* Repo header */}
            <button
                onClick={onToggle}
                className="w-full flex items-center gap-2 px-3 py-2 hover:bg-swarm-accent/5 transition-colors"
            >
                <svg
                    className={`w-3 h-3 text-swarm-text-dim transition-transform ${
                        group.expanded ? "rotate-90" : ""
                    }`}
                    fill="currentColor"
                    viewBox="0 0 20 20"
                >
                    <path d="M6 6L14 10L6 14V6Z" />
                </svg>
                <svg
                    className="w-3.5 h-3.5 text-swarm-text-dim"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    strokeWidth={1.5}
                >
                    <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        d="M2.25 12.75V12A2.25 2.25 0 0 1 4.5 9.75h15A2.25 2.25 0 0 1 21.75 12v.75m-8.69-6.44-2.12-2.12a1.5 1.5 0 0 0-1.061-.44H4.5A2.25 2.25 0 0 0 2.25 6v12a2.25 2.25 0 0 0 2.25 2.25h15A2.25 2.25 0 0 0 21.75 18V9a2.25 2.25 0 0 0-2.25-2.25h-5.379a1.5 1.5 0 0 1-1.06-.44Z"
                    />
                </svg>
                <span className="text-xs font-medium text-swarm-text truncate flex-1 text-left">
                    {getHighlightRanges ? (
                        <HighlightText text={group.name} ranges={getHighlightRanges(group.name)} />
                    ) : (
                        group.name
                    )}
                </span>
                <span className="text-[10px] text-swarm-text-dim tabular-nums">
                    {group.sessions.length}
                </span>
                {activeCount > 0 && (
                    <span className="w-1.5 h-1.5 rounded-full bg-green-400 animate-pulse" />
                )}
            </button>

            {/* Sessions under this repo */}
            {group.expanded && (
                <div className="pb-1">
                    {group.sessions.map((session) => (
                        <SessionCard
                            key={session.session_id}
                            session={session}
                            selected={selectedSessionId === session.session_id}
                            onClick={() => onSelectSession(session.session_id)}
                            getHighlightRanges={getHighlightRanges}
                        />
                    ))}
                </div>
            )}
        </div>
    );
}
