import { useEffect, useState, useMemo } from "react";
import { useSessionStore } from "../../stores/sessionStore";
import { SessionCard } from "./SessionCard";
import type { SessionInfo } from "../../types/session";

interface RepoGroup {
    name: string;
    path: string;
    sessions: SessionInfo[];
    expanded: boolean;
}

export function SessionList() {
    const {
        sessions,
        selectedSessionId,
        loading,
        fetchSessions,
        selectSession,
        startWatcher,
    } = useSessionStore();
    const [search, setSearch] = useState("");
    const [expandedRepos, setExpandedRepos] = useState<Set<string>>(new Set());

    useEffect(() => {
        fetchSessions();
        startWatcher();
    }, [fetchSessions, startWatcher]);

    // Group sessions by repository (project_path)
    const repoGroups = useMemo(() => {
        const groups = new Map<string, SessionInfo[]>();

        for (const session of sessions) {
            // Extract repo name from project_path
            const repoPath = session.project_path || session.encoded_path;
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
                    (a, b) => b.last_modified - a.last_modified,
                ),
                expanded: expandedRepos.has(path),
            });
        }

        // Sort repos by most recent session
        result.sort((a, b) => {
            const aMax = Math.max(...a.sessions.map((s) => s.last_modified));
            const bMax = Math.max(...b.sessions.map((s) => s.last_modified));
            return bMax - aMax;
        });

        return result;
    }, [sessions, expandedRepos]);

    // Filter by search
    const filteredGroups = useMemo(() => {
        if (!search.trim()) return repoGroups;
        const q = search.toLowerCase();
        return repoGroups
            .map((g) => ({
                ...g,
                sessions: g.sessions.filter(
                    (s) =>
                        s.project_path.toLowerCase().includes(q) ||
                        (s.git_branch &&
                            s.git_branch.toLowerCase().includes(q)) ||
                        s.id.toLowerCase().includes(q),
                ),
            }))
            .filter((g) => g.sessions.length > 0);
    }, [repoGroups, search]);

    const toggleRepo = (path: string) => {
        setExpandedRepos((prev) => {
            const next = new Set(prev);
            if (next.has(path)) next.delete(path);
            else next.add(path);
            return next;
        });
    };

    // Auto-expand repos with selected session or active sessions
    useEffect(() => {
        const newExpanded = new Set(expandedRepos);
        for (const group of repoGroups) {
            const hasSelected = group.sessions.some(
                (s) => s.id === selectedSessionId,
            );
            const hasActive = group.sessions.some(
                (s) =>
                    s.status.type === "thinking" ||
                    s.status.type === "executing_tool",
            );
            if (hasSelected || hasActive) {
                newExpanded.add(group.path);
            }
        }
        if (newExpanded.size !== expandedRepos.size) {
            setExpandedRepos(newExpanded);
        }
    }, [selectedSessionId, repoGroups]); // eslint-disable-line react-hooks/exhaustive-deps

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
}: {
    group: RepoGroup;
    selectedSessionId: string | null;
    onToggle: () => void;
    onSelectSession: (id: string | null) => void;
}) {
    const activeCount = group.sessions.filter(
        (s) =>
            s.status.type === "thinking" || s.status.type === "executing_tool",
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
                    {group.name}
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
                            key={session.jsonl_path}
                            session={session}
                            selected={selectedSessionId === session.id}
                            onClick={() => onSelectSession(session.id)}
                        />
                    ))}
                </div>
            )}
        </div>
    );
}
