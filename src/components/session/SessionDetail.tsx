import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, useCallback, useRef, useMemo } from "react";
import type { SessionInfo } from "../../types/session";
import { useSessionStore } from "../../stores/sessionStore";
import { HighlightText } from "./HighlightText";
import { ContextBar } from "./ContextBar";
import { SteeringInput } from "./SteeringInput";
import { SmithPanel } from "./SmithPanel";

interface SessionDetailProps {
    session: SessionInfo;
    onOpenTerminal?: (cwd: string, agentName?: string) => void;
    onResumeSession?: (sessionId: string, cwd: string) => void;
    onKillProcess?: (pid: number) => void;
    onShowDiff?: (repoPath: string) => void;
    onBack?: () => void;
}

interface ConversationMessage {
    role: string;
    content_type: string;
    text: string;
    tool_name: string | null;
    timestamp: number | null;
}

export function SessionDetail({
    session,
    onOpenTerminal: _onOpenTerminal,
    onResumeSession,
    onShowDiff,
    onBack,
}: SessionDetailProps) {
    void _onOpenTerminal; // Available for future terminal launch from detail view
    const searchQuery = useSessionStore(s => s.searchQuery);
    const [detail, setDetail] = useState<SessionInfo>(session);
    const [messages, setMessages] = useState<ConversationMessage[]>([]);
    const [showTools, setShowTools] = useState(false);
    const [showSmithPanel, setShowSmithPanel] = useState(false);
    const scrollRef = useRef<HTMLDivElement>(null);
    const isUserNearBottomRef = useRef(true);
    const isScrollingProgrammaticallyRef = useRef(false);
    // Match navigation
    const [currentMatchIdx, setCurrentMatchIdx] = useState(-1);
    const hasAutoScrolledRef = useRef(false);

    const handleScroll = useCallback(() => {
        if (isScrollingProgrammaticallyRef.current) return;
        const el = scrollRef.current;
        if (!el) return;
        const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
        isUserNearBottomRef.current = distanceFromBottom < 80;
    }, []);

    const refresh = useCallback(async () => {
        try {
            const info = await invoke<SessionInfo>("get_session_detail", {
                jsonlPath: session.jsonl_path,
            });
            setDetail(info);
        } catch {
            // Keep current data on error
        }
    }, [session.jsonl_path]);

    const fetchConversation = useCallback(async () => {
        try {
            const msgs = await invoke<ConversationMessage[]>(
                "get_conversation",
                {
                    jsonlPath: session.jsonl_path,
                },
            );
            setMessages(msgs);
        } catch {
            setMessages([]);
        }
    }, [session.jsonl_path]);

    useEffect(() => {
        hasAutoScrolledRef.current = false;
        refresh();
        fetchConversation();
        const interval = setInterval(() => {
            refresh();
            fetchConversation();
        }, 5000);
        return () => clearInterval(interval);
    }, [refresh, fetchConversation]);

    useEffect(() => {
        setDetail(session);
    }, [session]);

    // Auto-scroll to bottom only when user is near bottom AND no search active
    useEffect(() => {
        if (searchQuery.trim()) return; // Don't fight match navigation
        if (scrollRef.current && isUserNearBottomRef.current) {
            isScrollingProgrammaticallyRef.current = true;
            scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
            requestAnimationFrame(() => {
                isScrollingProgrammaticallyRef.current = false;
            });
        }
    }, [messages, searchQuery]);

    const cwd = detail.cwd || detail.project_path;

    // Compute highlight ranges for search query matches
    const searchTerms = useMemo(() => {
        if (!searchQuery.trim()) return null;
        const terms = searchQuery.trim().toLowerCase().split(/\s+/).filter(Boolean);
        return terms.length > 0 ? terms : null;
    }, [searchQuery]);

    const getHighlightRanges = useMemo(() => {
        if (!searchTerms) return null;
        return (text: string): { start: number; end: number }[] => {
            if (!text) return [];
            const ranges: { start: number; end: number }[] = [];
            const lower = text.toLowerCase();
            for (const term of searchTerms) {
                let pos = 0;
                while (pos < lower.length) {
                    const idx = lower.indexOf(term, pos);
                    if (idx === -1) break;
                    ranges.push({ start: idx, end: idx + term.length });
                    pos = idx + 1;
                }
            }
            ranges.sort((a, b) => a.start - b.start);
            const merged: { start: number; end: number }[] = [];
            for (const r of ranges) {
                const last = merged[merged.length - 1];
                if (last && r.start <= last.end) {
                    last.end = Math.max(last.end, r.end);
                } else {
                    merged.push({ ...r });
                }
            }
            return merged;
        };
    }, [searchTerms]);

    // Filter messages based on showTools toggle
    const displayMessages = showTools
        ? messages
        : messages.filter(
              (m) => m.content_type === "text" || m.content_type === "thinking",
          );

    // Indices of messages that contain search matches
    const matchingMsgIndices = useMemo(() => {
        if (!searchTerms) return [];
        const indices: number[] = [];
        for (let i = 0; i < displayMessages.length; i++) {
            const lower = displayMessages[i].text.toLowerCase();
            if (searchTerms.some(t => lower.includes(t))) {
                indices.push(i);
            }
        }
        return indices;
    }, [displayMessages, searchTerms]);

    const totalMatches = matchingMsgIndices.length;

    // Scroll to a specific match message element
    const scrollToMatch = useCallback((matchIdx: number) => {
        if (matchIdx < 0 || matchIdx >= matchingMsgIndices.length) return;
        const msgIndex = matchingMsgIndices[matchIdx];
        const container = scrollRef.current;
        if (!container) return;
        const el = container.querySelector(`[data-msg-idx="${msgIndex}"]`) as HTMLElement | null;
        if (!el) return;

        isScrollingProgrammaticallyRef.current = true;
        el.scrollIntoView({ block: "center", behavior: "smooth" });
        setCurrentMatchIdx(matchIdx);

        // Brief flash on the focused message
        el.classList.add("ring-1", "ring-amber-400/60");
        setTimeout(() => {
            el.classList.remove("ring-1", "ring-amber-400/60");
            isScrollingProgrammaticallyRef.current = false;
        }, 600);
    }, [matchingMsgIndices]);

    // Auto-scroll to last match when session first loads with a search query
    useEffect(() => {
        if (hasAutoScrolledRef.current) return;
        if (totalMatches === 0) return;
        // Wait a tick for DOM to render
        const timer = setTimeout(() => {
            scrollToMatch(totalMatches - 1);
            hasAutoScrolledRef.current = true;
        }, 150);
        return () => clearTimeout(timer);
    }, [totalMatches, scrollToMatch]);

    // Navigate: up = older match, down = newer match
    const goToPrevMatch = useCallback(() => {
        if (totalMatches === 0) return;
        const next = currentMatchIdx <= 0 ? totalMatches - 1 : currentMatchIdx - 1;
        scrollToMatch(next);
    }, [currentMatchIdx, totalMatches, scrollToMatch]);

    const goToNextMatch = useCallback(() => {
        if (totalMatches === 0) return;
        const next = currentMatchIdx >= totalMatches - 1 ? 0 : currentMatchIdx + 1;
        scrollToMatch(next);
    }, [currentMatchIdx, totalMatches, scrollToMatch]);

    // Keyboard shortcuts for match navigation
    useEffect(() => {
        if (totalMatches === 0) return;
        const handler = (e: KeyboardEvent) => {
            // Only capture when no input is focused
            const tag = (e.target as HTMLElement).tagName;
            if (tag === "INPUT" || tag === "TEXTAREA") return;

            if (e.key === "ArrowUp" || (e.key === "p" && (e.metaKey || e.ctrlKey))) {
                e.preventDefault();
                goToPrevMatch();
            } else if (e.key === "ArrowDown" || (e.key === "n" && (e.metaKey || e.ctrlKey))) {
                e.preventDefault();
                goToNextMatch();
            }
        };
        window.addEventListener("keydown", handler);
        return () => window.removeEventListener("keydown", handler);
    }, [totalMatches, goToPrevMatch, goToNextMatch]);

    return (
        <div className="flex flex-col h-full bg-swarm-surface overflow-hidden select-text">
            {/* Header bar */}
            <div className="shrink-0 border-b border-swarm-border">
                {/* Session ID + actions row */}
                <div className="flex items-center justify-between px-3 py-1.5 bg-swarm-bg">
                    <div className="flex items-center gap-2 min-w-0">
                        {onBack && (
                            <button
                                onClick={onBack}
                                className="px-1.5 py-0.5 text-[10px] text-swarm-text-dim border border-swarm-border rounded hover:text-swarm-text hover:border-swarm-accent/30 transition-colors mr-1 shrink-0"
                                title="Back to overview"
                            >
                                &larr;
                            </button>
                        )}
                        <StatusDot status={detail.status} />
                        <span className="text-[10px] font-mono text-swarm-text-dim truncate select-all" title={detail.id}>
                            {detail.id}
                        </span>
                    </div>
                    <div className="flex items-center gap-1 shrink-0">
                        {onShowDiff && (
                            <button
                                onClick={() => onShowDiff(cwd)}
                                className="px-2 py-0.5 text-[10px] text-swarm-text-dim border border-swarm-border rounded hover:text-swarm-text hover:border-swarm-accent/30 transition-colors"
                            >
                                Diff
                            </button>
                        )}
                        {onResumeSession && detail.status.type !== "thinking" && detail.status.type !== "executing_tool" && (
                            <button
                                onClick={() => onResumeSession(detail.id, cwd)}
                                className="px-2 py-0.5 text-[10px] text-swarm-accent border border-swarm-accent/30 rounded hover:bg-swarm-accent/10 transition-colors"
                                title={`Resume session ${detail.id}`}
                            >
                                Resume
                            </button>
                        )}
                    </div>
                </div>

                {/* Path + branch row */}
                <div className="flex items-center gap-2 px-3 py-1 text-xs">
                    <span className="text-swarm-text-dim font-mono truncate">
                        {cwd}
                    </span>
                    {detail.git_branch && (
                        <span className="text-swarm-accent shrink-0">
                            {detail.git_branch}
                        </span>
                    )}
                </div>

                {/* Context bar */}
                {(detail.context_tokens > 0 || detail.input_tokens > 0) && (
                    <div className="px-3 pb-1.5">
                        <ContextBar
                            contextTokens={detail.context_tokens}
                            inputTokens={detail.input_tokens}
                            cacheCreationTokens={detail.cache_creation_tokens}
                            cacheReadTokens={detail.cache_read_tokens}
                            model={detail.model}
                        />
                    </div>
                )}
            </div>

            {/* Match navigation bar */}
            {totalMatches > 0 && (
                <div className="shrink-0 flex items-center justify-between px-3 py-1 bg-amber-400/10 border-b border-amber-400/20">
                    <span className="text-[11px] text-amber-300 font-medium">
                        {currentMatchIdx >= 0 ? currentMatchIdx + 1 : "–"} / {totalMatches} matches
                    </span>
                    <div className="flex items-center gap-1">
                        <button
                            onClick={goToPrevMatch}
                            className="px-1.5 py-0.5 text-[11px] text-amber-300 border border-amber-400/30 rounded hover:bg-amber-400/20 transition-colors"
                            title="Previous match (↑)"
                        >
                            ↑
                        </button>
                        <button
                            onClick={goToNextMatch}
                            className="px-1.5 py-0.5 text-[11px] text-amber-300 border border-amber-400/30 rounded hover:bg-amber-400/20 transition-colors"
                            title="Next match (↓)"
                        >
                            ↓
                        </button>
                    </div>
                </div>
            )}

            {/* Conversation area - scrollable */}
            <div ref={scrollRef} onScroll={handleScroll} className="flex-1 min-h-0 overflow-y-auto">
                {/* Toggle for tool calls */}
                <div className="sticky top-0 z-10 flex items-center gap-2 px-3 py-1 bg-swarm-surface/95 backdrop-blur border-b border-swarm-border/50">
                    <label className="flex items-center gap-1.5 cursor-pointer text-[10px] text-swarm-text-dim">
                        <input
                            type="checkbox"
                            checked={showTools}
                            onChange={(e) => setShowTools(e.target.checked)}
                            className="w-3 h-3 accent-swarm-accent"
                        />
                        Show tool calls (
                        {
                            messages.filter(
                                (m) =>
                                    m.content_type !== "text" &&
                                    m.content_type !== "thinking",
                            ).length
                        }
                        )
                    </label>
                </div>

                {displayMessages.length === 0 ? (
                    <div className="flex items-center justify-center p-8 text-swarm-text-dim text-sm">
                        No conversation yet.
                    </div>
                ) : (
                    <div className="space-y-0">
                        {displayMessages.map((msg, i) => (
                            <MessageBubble
                                key={i}
                                msgIndex={i}
                                message={msg}
                                getHighlightRanges={getHighlightRanges}
                                isFocusedMatch={matchingMsgIndices[currentMatchIdx] === i}
                            />
                        ))}
                    </div>
                )}
            </div>

            {/* Smith override panel (slides up) */}
            {showSmithPanel && (
                <SmithPanel
                    sessionId={detail.id}
                    onClose={() => setShowSmithPanel(false)}
                />
            )}

            {/* Steering input */}
            <SteeringInput
                sessionId={detail.id}
                cwd={cwd}
                status={detail.status.type}
                onSmithConfig={() => setShowSmithPanel((v) => !v)}
            />

            {/* Model + token info footer */}
            <div className="shrink-0 flex items-center justify-end px-3 py-0.5 bg-swarm-bg border-t border-swarm-border/50">
                <div className="text-[10px] text-swarm-text-dim font-mono">
                    {detail.model ? formatModel(detail.model) : ""}
                    {detail.total_output_tokens > 0 &&
                        ` | ${formatTokens(detail.total_output_tokens)} out`}
                </div>
            </div>
        </div>
    );
}

function MessageBubble({ msgIndex, message, getHighlightRanges, isFocusedMatch }: {
    msgIndex: number;
    message: ConversationMessage;
    getHighlightRanges: ((text: string) => { start: number; end: number }[]) | null;
    isFocusedMatch: boolean;
}) {
    const isUser = message.role === "user";
    const isThinking = message.content_type === "thinking";
    const isTool =
        message.content_type === "tool_use" ||
        message.content_type === "tool_result";

    const renderText = (text: string, className?: string) => {
        if (getHighlightRanges) {
            const ranges = getHighlightRanges(text);
            if (ranges.length > 0) {
                return <HighlightText text={text} ranges={ranges} className={className} />;
            }
        }
        return <span className={className}>{text}</span>;
    };

    const focusClass = isFocusedMatch ? "bg-amber-400/5 border-l-2 border-amber-400/40" : "";

    if (isTool) {
        return (
            <div className={`px-3 py-1 transition-colors ${focusClass}`} data-msg-idx={msgIndex}>
                <div className="flex items-center gap-1.5 text-[10px]">
                    {message.content_type === "tool_use" ? (
                        <>
                            <span className="text-orange-400 font-medium">
                                {message.tool_name}
                            </span>
                            <span className="text-swarm-text-dim">
                                {renderText(
                                    message.text.slice(0, 80) + (message.text.length > 80 ? "..." : "")
                                )}
                            </span>
                        </>
                    ) : (
                        <>
                            <span className="text-green-400/70">result</span>
                            <span className="text-swarm-text-dim font-mono truncate">
                                {renderText(message.text.slice(0, 100))}
                            </span>
                        </>
                    )}
                </div>
            </div>
        );
    }

    if (isThinking) {
        return (
            <div className={`px-3 py-1.5 border-l-2 ${isFocusedMatch ? "border-amber-400/40" : "border-blue-500/30"} ml-3 my-1 transition-colors ${isFocusedMatch ? "bg-amber-400/5" : ""}`} data-msg-idx={msgIndex}>
                <div className="text-[10px] text-blue-400/60 mb-0.5 italic">
                    thinking
                </div>
                <div className="text-xs text-swarm-text-dim/70 italic whitespace-pre-wrap">
                    {renderText(message.text)}
                </div>
            </div>
        );
    }

    return (
        <div className={`px-3 py-2 transition-colors ${isUser ? "bg-swarm-bg/50" : ""} ${focusClass}`} data-msg-idx={msgIndex}>
            <div className="flex items-center gap-1.5 mb-1">
                <span
                    className={`w-1.5 h-1.5 rounded-full ${isUser ? "bg-blue-400" : "bg-swarm-accent"}`}
                />
                <span className="text-[10px] font-medium text-swarm-text-dim">
                    {isUser ? "You" : "Claude"}
                </span>
            </div>
            <div className="text-xs text-swarm-text whitespace-pre-wrap pl-3 leading-relaxed">
                {renderText(message.text)}
            </div>
        </div>
    );
}

function StatusDot({ status }: { status: SessionInfo["status"] }) {
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

function formatModel(model: string): string {
    const lower = model.toLowerCase();
    if (lower.includes("opus")) return "Opus 4.6";
    if (lower.includes("sonnet")) return "Sonnet 4.5";
    if (lower.includes("haiku")) return "Haiku 4.5";
    return model.split("-").slice(0, 2).join(" ");
}

function formatTokens(n: number): string {
    if (n === 0) return "-";
    if (n < 1000) return String(n);
    if (n < 1_000_000) return `${(n / 1000).toFixed(1)}K`;
    return `${(n / 1_000_000).toFixed(2)}M`;
}
