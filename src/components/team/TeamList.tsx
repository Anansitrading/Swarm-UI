import { useEffect, useState, useCallback } from "react";
import { useTeamStore } from "../../stores/teamStore";
import { useSessionStore } from "../../stores/sessionStore";
import { useLayoutStore } from "../../stores/layoutStore";
import type { TeamInfo, TeamTask, TeamMember } from "../../types/team";
import { taskStatusDot, taskStatusLabel } from "../../types/team";

export function TeamList() {
    const {
        teams,
        selectedTeamName,
        loading,
        fetchTeams,
        selectTeam,
        startWatcher,
    } = useTeamStore();

    useEffect(() => {
        fetchTeams();
        startWatcher();
    }, [fetchTeams, startWatcher]);

    if (loading && teams.length === 0) {
        return (
            <div className="flex items-center justify-center p-8 text-swarm-text-dim text-sm">
                Loading teams...
            </div>
        );
    }

    if (teams.length === 0) {
        return (
            <div className="p-4 text-center text-swarm-text-dim text-sm">
                <p>No agent teams found.</p>
                <p className="text-[10px] mt-2 opacity-60">
                    Teams appear in ~/.claude/teams/
                </p>
            </div>
        );
    }

    return (
        <div className="flex flex-col h-full">
            <div className="p-2 border-b border-swarm-border">
                <div className="text-[10px] text-swarm-text-dim flex items-center justify-between">
                    <span>
                        {teams.length} team{teams.length !== 1 ? "s" : ""}
                    </span>
                    <button
                        onClick={() => fetchTeams()}
                        className="hover:text-swarm-text transition-colors"
                    >
                        Refresh
                    </button>
                </div>
            </div>
            <div className="flex-1 overflow-y-auto">
                {teams.map((team) => (
                    <TeamCard
                        key={team.name}
                        team={team}
                        selected={selectedTeamName === team.name}
                        onSelect={() =>
                            selectTeam(
                                selectedTeamName === team.name
                                    ? null
                                    : team.name,
                            )
                        }
                    />
                ))}
            </div>
        </div>
    );
}

function TeamCard({
    team,
    selected,
    onSelect,
}: {
    team: TeamInfo;
    selected: boolean;
    onSelect: () => void;
}) {
    const [expanded, setExpanded] = useState(false);
    const hasActive = team.taskSummary.in_progress > 0;
    const timeAgo = team.createdAt ? formatTimeAgo(team.createdAt) : "";

    return (
        <div className="border-b border-swarm-border/50">
            {/* Team header */}
            <button
                onClick={() => {
                    onSelect();
                    setExpanded(!expanded);
                }}
                className={`w-full text-left px-3 py-2 transition-colors ${
                    selected
                        ? "bg-swarm-accent/10 border-l-2 border-swarm-accent"
                        : "hover:bg-swarm-accent/5 border-l-2 border-transparent"
                }`}
            >
                <div className="flex items-center gap-2">
                    {/* Expand arrow */}
                    <svg
                        className={`w-3 h-3 text-swarm-text-dim transition-transform shrink-0 ${
                            expanded ? "rotate-90" : ""
                        }`}
                        fill="currentColor"
                        viewBox="0 0 20 20"
                    >
                        <path d="M6 6L14 10L6 14V6Z" />
                    </svg>

                    {/* Team icon */}
                    <svg
                        className="w-3.5 h-3.5 text-purple-400 shrink-0"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        strokeWidth={1.5}
                    >
                        <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            d="M18 18.72a9.094 9.094 0 0 0 3.741-.479 3 3 0 0 0-4.682-2.72m.94 3.198.001.031c0 .225-.012.447-.037.666A11.944 11.944 0 0 1 12 21c-2.17 0-4.207-.576-5.963-1.584A6.062 6.062 0 0 1 6 18.719m12 0a5.971 5.971 0 0 0-.941-3.197m0 0A5.995 5.995 0 0 0 12 12.75a5.995 5.995 0 0 0-5.058 2.772m0 0a3 3 0 0 0-4.681 2.72 8.986 8.986 0 0 0 3.74.477m.94-3.197a5.971 5.971 0 0 0-.94 3.197M15 6.75a3 3 0 1 1-6 0 3 3 0 0 1 6 0Zm6 3a2.25 2.25 0 1 1-4.5 0 2.25 2.25 0 0 1 4.5 0Zm-13.5 0a2.25 2.25 0 1 1-4.5 0 2.25 2.25 0 0 1 4.5 0Z"
                        />
                    </svg>

                    {/* Team name */}
                    <span className="text-xs font-medium text-swarm-text truncate flex-1">
                        {team.name}
                    </span>

                    {/* Member count */}
                    <span className="text-[10px] text-swarm-text-dim tabular-nums shrink-0">
                        {team.members.length}
                    </span>

                    {/* Active indicator */}
                    {hasActive && (
                        <span className="relative flex h-1.5 w-1.5 shrink-0">
                            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-orange-400 opacity-75" />
                            <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-orange-400" />
                        </span>
                    )}
                </div>

                {/* Second line: description + time */}
                <div className="flex items-center justify-between mt-0.5 pl-5">
                    {team.description && (
                        <span className="text-[10px] text-swarm-text-dim truncate flex-1 mr-2">
                            {team.description}
                        </span>
                    )}
                    <span className="text-[10px] text-swarm-text-dim shrink-0">
                        {timeAgo}
                    </span>
                </div>

                {/* Task progress bar */}
                {team.taskSummary.total > 0 && (
                    <div className="mt-1.5 pl-5">
                        <div className="flex items-center gap-1.5">
                            <div className="flex-1 h-1 bg-swarm-border rounded-full overflow-hidden flex">
                                {team.taskSummary.completed > 0 && (
                                    <div
                                        className="h-full bg-green-400"
                                        style={{
                                            width: `${(team.taskSummary.completed / team.taskSummary.total) * 100}%`,
                                        }}
                                    />
                                )}
                                {team.taskSummary.in_progress > 0 && (
                                    <div
                                        className="h-full bg-orange-400"
                                        style={{
                                            width: `${(team.taskSummary.in_progress / team.taskSummary.total) * 100}%`,
                                        }}
                                    />
                                )}
                            </div>
                            <span className="text-[9px] text-swarm-text-dim tabular-nums">
                                {team.taskSummary.completed}/
                                {team.taskSummary.total}
                            </span>
                        </div>
                    </div>
                )}
            </button>

            {/* Expanded details */}
            {expanded && (
                <div className="px-3 pb-2 space-y-2">
                    {/* Members */}
                    <MembersList members={team.members} />

                    {/* Tasks */}
                    {team.tasks.length > 0 && <TasksList tasks={team.tasks} />}
                </div>
            )}
        </div>
    );
}

function MembersList({ members }: { members: TeamMember[] }) {
    // Use selectors to avoid re-rendering on every session update.
    // Only subscribe to selectSession function (stable reference) and setMode.
    const selectSession = useSessionStore((s) => s.selectSession);
    const setMode = useLayoutStore((s) => s.setMode);

    const handleMemberClick = useCallback(
        (member: TeamMember) => {
            // Lazily read sessions at click time instead of subscribing
            const sessions = useSessionStore.getState().sessions;
            const match = sessions.find(
                (s) =>
                    s.id === member.agentId ||
                    s.id.startsWith(member.agentId.slice(0, 8)),
            );
            if (match) {
                selectSession(match.id);
                setMode("single", false);
            }
        },
        [selectSession, setMode],
    );

    return (
        <div>
            <div className="text-[10px] text-swarm-text-dim font-medium uppercase tracking-wider mb-1 pl-5">
                Members ({members.length})
            </div>
            {members.map((member) => (
                <button
                    key={member.agentId}
                    onClick={() => handleMemberClick(member)}
                    className="w-full flex items-center gap-2 pl-5 py-0.5 text-left transition-colors hover:bg-swarm-accent/10 cursor-pointer"
                    title="Click to view session"
                >
                    {/* Role icon */}
                    {member.agentType === "lead" ||
                    member.agentType === "team-lead" ? (
                        <svg
                            className="w-3 h-3 text-yellow-400 shrink-0"
                            fill="currentColor"
                            viewBox="0 0 20 20"
                        >
                            <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z" />
                        </svg>
                    ) : (
                        <svg
                            className="w-3 h-3 text-swarm-text-dim shrink-0"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                            strokeWidth={1.5}
                        >
                            <path
                                strokeLinecap="round"
                                strokeLinejoin="round"
                                d="M15.75 6a3.75 3.75 0 1 1-7.5 0 3.75 3.75 0 0 1 7.5 0ZM4.501 20.118a7.5 7.5 0 0 1 14.998 0"
                            />
                        </svg>
                    )}
                    <span className="text-[11px] text-swarm-text truncate flex-1">
                        {member.name}
                    </span>
                    {member.model && (
                        <span className="text-[9px] text-swarm-text-dim font-mono shrink-0">
                            {formatModel(member.model)}
                        </span>
                    )}
                </button>
            ))}
        </div>
    );
}

function TasksList({ tasks }: { tasks: TeamTask[] }) {
    const [showAll, setShowAll] = useState(false);
    const visibleTasks = showAll ? tasks : tasks.slice(0, 6);
    const hasMore = tasks.length > 6;

    return (
        <div>
            <div className="text-[10px] text-swarm-text-dim font-medium uppercase tracking-wider mb-1 pl-5">
                Tasks ({tasks.length})
            </div>
            {visibleTasks.map((task) => (
                <div
                    key={task.id}
                    className="flex items-start gap-2 pl-5 py-0.5 group"
                    title={task.description || task.subject || ""}
                >
                    {/* Status dot */}
                    <span
                        className={`mt-1 inline-flex rounded-full h-1.5 w-1.5 shrink-0 ${taskStatusDot(task.status)}`}
                    />
                    {/* Task info */}
                    <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-1">
                            <span className="text-[10px] text-swarm-text-dim tabular-nums shrink-0">
                                #{task.id}
                            </span>
                            <span
                                className={`text-[11px] truncate ${
                                    task.status === "completed"
                                        ? "text-swarm-text-dim line-through"
                                        : "text-swarm-text"
                                }`}
                            >
                                {task.subject || task.activeForm || "Untitled"}
                            </span>
                        </div>
                        {task.owner && (
                            <span className="text-[9px] text-swarm-text-dim">
                                {task.owner}
                            </span>
                        )}
                    </div>
                    {/* Status label */}
                    <span className="text-[9px] text-swarm-text-dim shrink-0 mt-0.5">
                        {taskStatusLabel(task.status)}
                    </span>
                </div>
            ))}
            {hasMore && (
                <button
                    onClick={() => setShowAll(!showAll)}
                    className="text-[10px] text-swarm-accent hover:text-swarm-accent/80 pl-5 mt-1"
                >
                    {showAll ? "Show less" : `+${tasks.length - 6} more`}
                </button>
            )}
        </div>
    );
}

function formatModel(model: string): string {
    const lower = model.toLowerCase();
    if (lower.includes("opus")) return "Opus";
    if (lower.includes("sonnet")) return "Sonnet";
    if (lower.includes("haiku")) return "Haiku";
    return model.split("-")[0] ?? model;
}

function formatTimeAgo(epochMs: number): string {
    if (!epochMs) return "";
    const now = Date.now();
    const diff = (now - epochMs) / 1000;
    if (diff < 60) return "just now";
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
}
