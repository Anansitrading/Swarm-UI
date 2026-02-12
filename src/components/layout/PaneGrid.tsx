import { useCallback, useState } from "react";
import { useLayoutStore } from "../../stores/layoutStore";
import { useSessionStore } from "../../stores/sessionStore";
import { useTerminalStore } from "../../stores/terminalStore";
import { TerminalPane } from "../terminal/TerminalPane";
import { SessionDetail } from "../session/SessionDetail";
import { SpriteGrid } from "../sprite/SpriteGrid";
import { DiffViewer } from "../diff/DiffViewer";
import { AgentPicker } from "../terminal/AgentPicker";
import type { LayoutMode } from "../../types/terminal";
import { invoke } from "@tauri-apps/api/core";

/** Build the shell command to launch an agent session */
function agentCommand(agentName: string): { shell: string; args: string[] } {
    if (agentName === "claude") {
        return {
            shell: "claude",
            args: ["--dangerously-skip-permissions", "--chrome"],
        };
    }
    return {
        shell: "claude",
        args: [
            "--dangerously-skip-permissions",
            "--chrome",
            "--agent",
            agentName,
        ],
    };
}

interface GitFileChange {
    path: string;
    status: string;
    staged: boolean;
}

interface GitCommit {
    hash: string;
    short_hash: string;
    author: string;
    time_ago: string;
    subject: string;
}

interface FileDiffContent {
    diff: string;
    old_content: string;
    new_content: string;
}

function gridStyle(mode: LayoutMode): React.CSSProperties {
    switch (mode) {
        case "single":
            return { gridTemplateColumns: "1fr" };
        case "list":
            return { gridTemplateColumns: "1fr" };
        case "two_column":
            return { gridTemplateColumns: "1fr 1fr" };
        case "three_column":
            return { gridTemplateColumns: "1fr 1fr 1fr" };
        case "sprite_grid":
            return {};
    }
}

export function PaneGrid() {
    const { mode, panes, addPane, updatePane } = useLayoutStore();
    const { selectedSessionId, sessions, selectSession } = useSessionStore();
    const { spawnTerminal } = useTerminalStore();
    const [diffState, setDiffState] = useState<{
        repoPath: string;
        changes: GitFileChange[];
        commits: GitCommit[];
        selectedFile: string | null;
        oldContent: string;
        newContent: string;
    } | null>(null);

    const selectedSession = sessions.find((s) => s.id === selectedSessionId);

    const handleOpenTerminal = useCallback(
        async (cwd: string, agentName?: string) => {
            try {
                let config: Record<string, unknown> = { cwd };
                if (agentName) {
                    const cmd = agentCommand(agentName);
                    config = { ...config, shell: cmd.shell, args: cmd.args };
                }
                const info = await spawnTerminal(config);
                if (panes.length > 0) {
                    updatePane(0, { terminalId: info.id, type: "terminal" });
                } else {
                    addPane({
                        id: `terminal-${info.id}`,
                        type: "terminal",
                        terminalId: info.id,
                    });
                }
            } catch (e) {
                console.error("Failed to open terminal:", e);
            }
        },
        [spawnTerminal, addPane, panes, updatePane],
    );

    const handleResumeSession = useCallback(
        async (sessionId: string, cwd: string) => {
            try {
                const info = await spawnTerminal({
                    shell: "claude",
                    args: ["--resume", sessionId, "--dangerously-skip-permissions"],
                    cwd,
                });
                if (panes.length > 0) {
                    updatePane(0, { terminalId: info.id, type: "terminal" });
                } else {
                    addPane({
                        id: `terminal-${info.id}`,
                        type: "terminal",
                        terminalId: info.id,
                    });
                }
            } catch (e) {
                console.error("Failed to resume session:", e);
            }
        },
        [spawnTerminal, addPane, panes, updatePane],
    );

    const handleKillProcess = useCallback(async (pid: number) => {
        try {
            await invoke("kill_process", { pid, force: false });
        } catch (e) {
            console.error("Failed to kill process:", e);
        }
    }, []);

    const handleBack = useCallback(() => {
        selectSession(null);
    }, [selectSession]);

    const [diffError, setDiffError] = useState<string | null>(null);

    const handleShowDiff = useCallback(async (repoPath: string) => {
        if (!repoPath) {
            setDiffError("No working directory available for this session.");
            setTimeout(() => setDiffError(null), 3000);
            return;
        }
        try {
            // First check if it's a git repo by trying to get the branch
            const branch = await invoke<string | null>("get_git_branch", {
                path: repoPath,
            });
            if (branch === null) {
                setDiffError(`Not a git repository: ${repoPath}`);
                setTimeout(() => setDiffError(null), 3000);
                return;
            }
            // Fetch changes (includes untracked) and recent commits in parallel
            const [changes, commits] = await Promise.all([
                invoke<GitFileChange[]>("get_git_diff", { repoPath }),
                invoke<GitCommit[]>("get_git_log", { repoPath, count: 10 }),
            ]);
            if (changes.length === 0 && commits.length === 0) {
                setDiffError("No changes, untracked files, or commits found.");
                setTimeout(() => setDiffError(null), 3000);
                return;
            }
            setDiffError(null);
            setDiffState({
                repoPath,
                changes,
                commits,
                selectedFile: null,
                oldContent: "",
                newContent: "",
            });
        } catch (e) {
            setDiffError(`Diff failed: ${e}`);
            setTimeout(() => setDiffError(null), 4000);
            console.error("Failed to get diff:", e);
        }
    }, []);

    const handleSelectDiffFile = useCallback(
        async (filePath: string, staged: boolean) => {
            if (!diffState) return;
            try {
                const content = await invoke<FileDiffContent>("get_file_diff", {
                    repoPath: diffState.repoPath,
                    filePath,
                    staged,
                });
                setDiffState((prev) =>
                    prev
                        ? {
                              ...prev,
                              selectedFile: filePath,
                              oldContent: content.old_content,
                              newContent: content.new_content,
                          }
                        : null,
                );
            } catch (e) {
                console.error("Failed to get file diff:", e);
            }
        },
        [diffState],
    );

    const handleSelectCommitFile = useCallback(
        async (commitHash: string, filePath: string) => {
            if (!diffState) return;
            try {
                const content = await invoke<FileDiffContent>("get_commit_file_diff", {
                    repoPath: diffState.repoPath,
                    commitHash,
                    filePath,
                });
                setDiffState((prev) =>
                    prev
                        ? {
                              ...prev,
                              selectedFile: `${commitHash}:${filePath}`,
                              oldContent: content.old_content,
                              newContent: content.new_content,
                          }
                        : null,
                );
            } catch (e) {
                console.error("Failed to get commit file diff:", e);
            }
        },
        [diffState],
    );

    if (mode === "sprite_grid") {
        return (
            <div className="flex-1 min-h-0 overflow-hidden">
                <SpriteGrid />
            </div>
        );
    }

    // In "single" mode - if diff is open, show diff; if session selected, show detail; else terminal
    if (mode === "single") {
        if (diffState) {
            return (
                <div className="flex-1 min-h-0 flex overflow-hidden">
                    <DiffSidebar
                        changes={diffState.changes}
                        commits={diffState.commits}
                        selectedFile={diffState.selectedFile}
                        repoPath={diffState.repoPath}
                        onSelectFile={handleSelectDiffFile}
                        onSelectCommitFile={handleSelectCommitFile}
                        onClose={() => setDiffState(null)}
                    />
                    <div className="flex-1 min-h-0 overflow-hidden">
                        {diffState.selectedFile ? (
                            <DiffViewer
                                oldContent={diffState.oldContent}
                                newContent={diffState.newContent}
                                fileName={diffState.selectedFile.includes(":") ? diffState.selectedFile.split(":").slice(1).join(":") : diffState.selectedFile}
                            />
                        ) : (
                            <div className="flex items-center justify-center h-full text-swarm-text-dim text-sm">
                                Select a file to view diff
                            </div>
                        )}
                    </div>
                </div>
            );
        }

        if (selectedSession) {
            return (
                <div className="flex-1 min-h-0 overflow-hidden relative">
                    {diffError && <DiffErrorToast message={diffError} />}
                    <SessionDetail
                        session={selectedSession}
                        onOpenTerminal={handleOpenTerminal}
                        onResumeSession={handleResumeSession}
                        onKillProcess={handleKillProcess}
                        onShowDiff={handleShowDiff}
                        onBack={handleBack}
                    />
                </div>
            );
        }

        // No session selected - show terminal if one is open, else monitoring overview
        if (panes.length > 0 && panes[0].terminalId) {
            return (
                <div className="flex-1 min-h-0 overflow-hidden">
                    <TerminalPane key={panes[0].terminalId} ptyId={panes[0].terminalId} />
                </div>
            );
        }

        return (
            <div className="flex-1 min-h-0 overflow-y-auto p-2">
                <MonitoringOverview
                    sessions={sessions}
                    onOpenTerminal={handleOpenTerminal}
                    onKillProcess={handleKillProcess}
                    onShowDiff={handleShowDiff}
                />
            </div>
        );
    }

    // In "list" mode - session detail + terminal side by side (like AgentHub)
    if (mode === "list") {
        if (diffState) {
            return (
                <div className="flex-1 min-h-0 flex overflow-hidden">
                    <DiffSidebar
                        changes={diffState.changes}
                        commits={diffState.commits}
                        selectedFile={diffState.selectedFile}
                        repoPath={diffState.repoPath}
                        onSelectFile={handleSelectDiffFile}
                        onSelectCommitFile={handleSelectCommitFile}
                        onClose={() => setDiffState(null)}
                    />
                    <div className="flex-1 min-h-0 overflow-hidden">
                        {diffState.selectedFile ? (
                            <DiffViewer
                                oldContent={diffState.oldContent}
                                newContent={diffState.newContent}
                                fileName={diffState.selectedFile.includes(":") ? diffState.selectedFile.split(":").slice(1).join(":") : diffState.selectedFile}
                            />
                        ) : (
                            <div className="flex items-center justify-center h-full text-swarm-text-dim text-sm">
                                Select a file to view diff
                            </div>
                        )}
                    </div>
                </div>
            );
        }

        if (selectedSession) {
            return (
                <div className="flex-1 min-h-0 grid grid-cols-2 gap-0 overflow-hidden relative">
                    {diffError && <DiffErrorToast message={diffError} />}
                    <div className="min-h-0 border-r border-swarm-border overflow-hidden">
                        <SessionDetail
                            session={selectedSession}
                            onOpenTerminal={handleOpenTerminal}
                            onResumeSession={handleResumeSession}
                            onKillProcess={handleKillProcess}
                            onShowDiff={handleShowDiff}
                            onBack={handleBack}
                        />
                    </div>
                    <div className="min-h-0 overflow-hidden">
                        {panes.length > 0 && panes[0].terminalId ? (
                            <TerminalPane key={panes[0].terminalId} ptyId={panes[0].terminalId} />
                        ) : (
                            <AgentLauncher
                                cwd={
                                    selectedSession.cwd ||
                                    selectedSession.project_path
                                }
                                onLaunch={handleOpenTerminal}
                            />
                        )}
                    </div>
                </div>
            );
        }

        // No session selected - show terminal if one is open, else monitoring overview
        if (panes.length > 0 && panes[0].terminalId) {
            return (
                <div className="flex-1 min-h-0 overflow-hidden">
                    <TerminalPane key={panes[0].terminalId} ptyId={panes[0].terminalId} />
                </div>
            );
        }

        return (
            <div className="flex-1 min-h-0 overflow-y-auto p-2">
                <MonitoringOverview
                    sessions={sessions}
                    onOpenTerminal={handleOpenTerminal}
                    onKillProcess={handleKillProcess}
                    onShowDiff={handleShowDiff}
                />
            </div>
        );
    }

    // Multi-column modes - show session detail cards or terminals
    return (
        <div
            className="flex-1 min-h-0 grid gap-1 p-1 overflow-hidden"
            style={gridStyle(mode)}
        >
            {mode === "two_column" || mode === "three_column" ? (
                // Show active/recent sessions as monitoring cards
                <MultiSessionView
                    mode={mode}
                    sessions={sessions}
                    onOpenTerminal={handleOpenTerminal}
                    onResumeSession={handleResumeSession}
                    onKillProcess={handleKillProcess}
                    onShowDiff={handleShowDiff}
                />
            ) : (
                panes.map((pane) => (
                    <div
                        key={pane.id}
                        className="min-h-0 rounded border border-swarm-border overflow-hidden"
                    >
                        {pane.type === "terminal" && (
                            <TerminalPane ptyId={pane.terminalId} />
                        )}
                        {pane.type === "empty" && (
                            <div className="flex items-center justify-center h-full text-swarm-text-dim text-sm">
                                Empty pane
                            </div>
                        )}
                    </div>
                ))
            )}
        </div>
    );
}

/** Inline agent launcher with agent picker for the "No terminal open" state */
function AgentLauncher({
    cwd,
    onLaunch,
}: {
    cwd: string;
    onLaunch: (cwd: string, agent?: string) => void;
}) {
    const [showPicker, setShowPicker] = useState(false);

    return (
        <div className="relative flex flex-col items-center justify-center h-full bg-swarm-bg gap-3">
            <span className="text-swarm-text-dim text-sm">
                No terminal open
            </span>
            <button
                onClick={() => setShowPicker((v) => !v)}
                className="px-4 py-2 text-xs bg-swarm-accent/20 text-swarm-accent border border-swarm-accent/30 rounded hover:bg-swarm-accent/30 transition-colors"
            >
                Launch Agent Session
            </button>
            {showPicker && (
                <AgentPicker
                    position="inline"
                    onSelect={(agent) => {
                        setShowPicker(false);
                        onLaunch(cwd, agent);
                    }}
                    onClose={() => setShowPicker(false)}
                />
            )}
        </div>
    );
}

/** Shows multiple sessions as monitoring cards in 2/3 column layout */
function MultiSessionView({
    mode,
    sessions,
    onOpenTerminal,
    onResumeSession,
    onKillProcess,
    onShowDiff,
}: {
    mode: LayoutMode;
    sessions: import("../../types/session").SessionInfo[];
    onOpenTerminal: (cwd: string) => void;
    onResumeSession: (sessionId: string, cwd: string) => void;
    onKillProcess: (pid: number) => void;
    onShowDiff: (repoPath: string) => void;
}) {
    const count = mode === "three_column" ? 3 : 2;
    // Show the most recent sessions
    const recentSessions = sessions.slice(0, count);

    return (
        <>
            {recentSessions.map((session) => (
                <div
                    key={session.id}
                    className="min-h-0 rounded border border-swarm-border overflow-hidden"
                >
                    <SessionDetail
                        session={session}
                        onOpenTerminal={onOpenTerminal}
                        onResumeSession={onResumeSession}
                        onKillProcess={onKillProcess}
                        onShowDiff={onShowDiff}
                    />
                </div>
            ))}
            {/* Fill remaining slots with empty panes */}
            {Array.from({
                length: Math.max(0, count - recentSessions.length),
            }).map((_, i) => (
                <div
                    key={`empty-${i}`}
                    className="min-h-0 rounded border border-swarm-border overflow-hidden flex items-center justify-center text-swarm-text-dim text-sm"
                >
                    No session
                </div>
            ))}
        </>
    );
}

/** Overview grid showing all sessions as compact cards */
function MonitoringOverview({
    sessions,
}: {
    sessions: import("../../types/session").SessionInfo[];
    onOpenTerminal?: (cwd: string) => void;
    onKillProcess?: (pid: number) => void;
    onShowDiff?: (repoPath: string) => void;
}) {
    const { selectSession, selectedSessionId } = useSessionStore();

    if (sessions.length === 0) {
        return (
            <div className="flex items-center justify-center h-full text-swarm-text-dim text-sm">
                No active sessions. Start a Claude Code session to monitor it
                here.
            </div>
        );
    }

    return (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-2">
            {sessions.slice(0, 9).map((session) => {
                const name = session.project_path.split("/").pop() || "Unknown";
                const isActive =
                    session.status.type === "thinking" ||
                    session.status.type === "executing_tool";

                return (
                    <button
                        key={session.id}
                        onClick={() =>
                            selectSession(
                                selectedSessionId === session.id
                                    ? null
                                    : session.id,
                            )
                        }
                        className={`text-left p-3 rounded-lg border transition-colors ${
                            isActive
                                ? "border-swarm-accent/30 bg-swarm-accent/5"
                                : "border-swarm-border bg-swarm-surface hover:border-swarm-accent/20"
                        }`}
                    >
                        <div className="flex items-center justify-between mb-2">
                            <span className="text-sm font-medium text-swarm-text truncate">
                                {name}
                            </span>
                            <StatusDot status={session.status} />
                        </div>
                        {session.git_branch && (
                            <div className="text-[10px] text-swarm-accent mb-1">
                                {session.git_branch}
                            </div>
                        )}
                        {(session.context_tokens > 0 ||
                            session.input_tokens > 0) && (
                            <div className="mb-1">
                                <div className="flex justify-between text-[10px] text-swarm-text-dim mb-0.5">
                                    <span>
                                        {formatTokens(
                                            session.context_tokens ||
                                                session.input_tokens,
                                        )}
                                        /{formatTokens(200000)}
                                    </span>
                                    <span>
                                        {(
                                            ((session.context_tokens ||
                                                session.input_tokens) /
                                                200000) *
                                            100
                                        ).toFixed(0)}
                                        %
                                    </span>
                                </div>
                                <div className="h-1 bg-swarm-border rounded-full overflow-hidden">
                                    <div
                                        className="h-full bg-swarm-accent rounded-full"
                                        style={{
                                            width: `${Math.min(((session.context_tokens || session.input_tokens) / 200000) * 100, 100)}%`,
                                        }}
                                    />
                                </div>
                            </div>
                        )}
                        <div className="flex items-center justify-between text-[10px] text-swarm-text-dim">
                            <span>
                                {session.model
                                    ? formatModel(session.model)
                                    : ""}
                            </span>
                            <span>{formatTimeAgo(session.last_modified)}</span>
                        </div>
                    </button>
                );
            })}
        </div>
    );
}

function StatusDot({
    status,
}: {
    status: import("../../types/session").SessionStatus;
}) {
    const colors: Record<string, string> = {
        thinking: "bg-blue-400",
        executing_tool: "bg-orange-400",
        awaiting_approval: "bg-yellow-400",
        waiting: "bg-blue-300",
        idle: "bg-gray-400",
        stopped: "bg-red-400",
        unknown: "bg-gray-500",
    };
    const color = colors[status.type] || "bg-gray-500";
    const isActive =
        status.type === "thinking" || status.type === "executing_tool";

    return (
        <span className="relative flex h-2 w-2">
            {isActive && (
                <span
                    className={`animate-ping absolute inline-flex h-full w-full rounded-full ${color} opacity-75`}
                />
            )}
            <span
                className={`relative inline-flex rounded-full h-2 w-2 ${color}`}
            />
        </span>
    );
}

function DiffSidebar({
    changes,
    commits,
    selectedFile,
    repoPath,
    onSelectFile,
    onSelectCommitFile,
    onClose,
}: {
    changes: GitFileChange[];
    commits: GitCommit[];
    selectedFile: string | null;
    repoPath: string;
    onSelectFile: (path: string, staged: boolean) => void;
    onSelectCommitFile: (commitHash: string, filePath: string) => void;
    onClose: () => void;
}) {
    const trackedChanges = changes.filter((c) => c.status !== "untracked");
    const untrackedFiles = changes.filter((c) => c.status === "untracked");
    const [expandedCommit, setExpandedCommit] = useState<string | null>(null);
    const [commitFiles, setCommitFiles] = useState<GitFileChange[]>([]);

    const handleExpandCommit = useCallback(async (hash: string) => {
        if (expandedCommit === hash) {
            setExpandedCommit(null);
            setCommitFiles([]);
            return;
        }
        try {
            const files = await invoke<GitFileChange[]>("get_commit_files", {
                repoPath,
                commitHash: hash,
            });
            setCommitFiles(files);
            setExpandedCommit(hash);
        } catch (e) {
            console.error("Failed to get commit files:", e);
        }
    }, [expandedCommit, repoPath]);

    return (
        <div className="w-64 flex flex-col bg-swarm-surface border-r border-swarm-border overflow-hidden">
            <div className="flex items-center justify-between px-3 py-2 border-b border-swarm-border">
                <span className="text-xs font-medium text-swarm-text">
                    Git Changes
                </span>
                <button
                    onClick={onClose}
                    className="text-swarm-text-dim hover:text-swarm-text text-xs"
                >
                    Close
                </button>
            </div>
            <div className="flex-1 overflow-y-auto">
                {/* Working tree changes */}
                {trackedChanges.length > 0 && (
                    <div>
                        <div className="px-3 py-1.5 text-[10px] font-medium text-swarm-accent uppercase tracking-wider bg-swarm-bg/50">
                            Working Tree ({trackedChanges.length})
                        </div>
                        {trackedChanges.map((change) => (
                            <button
                                key={`${change.path}-${change.staged}`}
                                onClick={() => onSelectFile(change.path, change.staged)}
                                className={`w-full text-left px-3 py-1.5 text-xs flex items-center gap-2 hover:bg-swarm-accent/5 transition-colors ${
                                    selectedFile === change.path ? "bg-swarm-accent/10" : ""
                                }`}
                            >
                                <StatusIcon status={change.status} />
                                <span className="truncate text-swarm-text">
                                    {change.path.split("/").pop()}
                                </span>
                                {change.staged && (
                                    <span className="text-[9px] text-green-400 shrink-0">staged</span>
                                )}
                            </button>
                        ))}
                    </div>
                )}

                {/* Untracked files */}
                {untrackedFiles.length > 0 && (
                    <div>
                        <div className="px-3 py-1.5 text-[10px] font-medium text-yellow-400 uppercase tracking-wider bg-swarm-bg/50">
                            Untracked ({untrackedFiles.length})
                        </div>
                        {untrackedFiles.map((file) => (
                            <button
                                key={file.path}
                                onClick={() => onSelectFile(file.path, false)}
                                className={`w-full text-left px-3 py-1.5 text-xs flex items-center gap-2 hover:bg-swarm-accent/5 transition-colors ${
                                    selectedFile === file.path ? "bg-swarm-accent/10" : ""
                                }`}
                            >
                                <span className="text-[10px] font-mono font-bold text-cyan-400">?</span>
                                <span className="truncate text-swarm-text">
                                    {file.path.split("/").pop()}
                                </span>
                            </button>
                        ))}
                    </div>
                )}

                {/* Commits - clickable to expand and show files */}
                {commits.length > 0 && (
                    <div>
                        <div className="px-3 py-1.5 text-[10px] font-medium text-blue-400 uppercase tracking-wider bg-swarm-bg/50">
                            Commits ({commits.length})
                        </div>
                        {commits.map((commit) => (
                            <div key={commit.hash}>
                                <button
                                    onClick={() => handleExpandCommit(commit.hash)}
                                    className={`w-full text-left px-3 py-1.5 text-xs border-b border-swarm-border/30 hover:bg-swarm-accent/5 transition-colors ${
                                        expandedCommit === commit.hash ? "bg-swarm-accent/10" : ""
                                    }`}
                                >
                                    <div className="flex items-center gap-2">
                                        <svg
                                            className={`w-2.5 h-2.5 text-swarm-text-dim transition-transform shrink-0 ${
                                                expandedCommit === commit.hash ? "rotate-90" : ""
                                            }`}
                                            fill="currentColor"
                                            viewBox="0 0 20 20"
                                        >
                                            <path d="M6 6L14 10L6 14V6Z" />
                                        </svg>
                                        <span className="font-mono text-swarm-accent text-[10px] shrink-0">
                                            {commit.short_hash}
                                        </span>
                                        <span className="truncate text-swarm-text">
                                            {commit.subject}
                                        </span>
                                    </div>
                                    <div className="text-[10px] text-swarm-text-dim mt-0.5 pl-4">
                                        {commit.author} &middot; {commit.time_ago}
                                    </div>
                                </button>
                                {expandedCommit === commit.hash && commitFiles.length > 0 && (
                                    <div className="bg-swarm-bg/30">
                                        {commitFiles.map((file) => (
                                            <button
                                                key={file.path}
                                                onClick={() => onSelectCommitFile(commit.hash, file.path)}
                                                className={`w-full text-left pl-8 pr-3 py-1 text-xs flex items-center gap-2 hover:bg-swarm-accent/5 transition-colors ${
                                                    selectedFile === `${commit.hash}:${file.path}` ? "bg-swarm-accent/10" : ""
                                                }`}
                                            >
                                                <StatusIcon status={file.status} />
                                                <span className="truncate text-swarm-text">
                                                    {file.path.split("/").pop()}
                                                </span>
                                            </button>
                                        ))}
                                    </div>
                                )}
                            </div>
                        ))}
                    </div>
                )}

                {trackedChanges.length === 0 && untrackedFiles.length === 0 && commits.length === 0 && (
                    <div className="p-3 text-xs text-swarm-text-dim">
                        No changes or commits
                    </div>
                )}
            </div>
        </div>
    );
}

function StatusIcon({ status }: { status: string }) {
    const colors: Record<string, string> = {
        modified: "text-yellow-400",
        added: "text-green-400",
        deleted: "text-red-400",
        renamed: "text-blue-400",
        untracked: "text-cyan-400",
    };
    const letters: Record<string, string> = {
        modified: "M",
        added: "A",
        deleted: "D",
        renamed: "R",
        untracked: "?",
    };
    return (
        <span
            className={`text-[10px] font-mono font-bold ${colors[status] || "text-gray-400"}`}
        >
            {letters[status] || "?"}
        </span>
    );
}

function formatModel(model: string): string {
    const lower = model.toLowerCase();
    if (lower.includes("opus")) return "Opus";
    if (lower.includes("sonnet")) return "Sonnet";
    if (lower.includes("haiku")) return "Haiku";
    return model.split("-")[0] ?? model;
}

function formatTokens(n: number): string {
    if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
    return String(n);
}

function formatTimeAgo(epochSec: number): string {
    if (!epochSec) return "";
    const now = Date.now() / 1000;
    const diff = now - epochSec;
    if (diff < 60) return "just now";
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
}

function DiffErrorToast({ message }: { message: string }) {
    return (
        <div className="absolute top-2 left-1/2 -translate-x-1/2 z-50 px-4 py-2 bg-red-500/90 text-white text-xs rounded-lg shadow-lg animate-pulse">
            {message}
        </div>
    );
}
