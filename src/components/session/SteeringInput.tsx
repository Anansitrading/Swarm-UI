import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useState, useRef, useCallback, useEffect } from "react";

interface SteeringInputProps {
    sessionId: string;
    cwd: string;
    status: string; // status.type from SessionStatus
    onSmithConfig?: () => void;
}

// @ reference types available in Claude Code
const AT_REFERENCES = [
    {
        label: "File",
        prefix: "@file:",
        description: "Attach file contents",
        icon: "📄",
    },
    {
        label: "Rules",
        prefix: "@rules",
        description: "Insert CLAUDE.md rules",
        icon: "📋",
    },
    {
        label: "Fetch",
        prefix: "@fetch:",
        description: "Fetch URL content",
        icon: "🌐",
    },
    {
        label: "Diagnostics",
        prefix: "@diag",
        description: "Insert recent errors",
        icon: "🔍",
    },
];

export function SteeringInput({
    sessionId,
    cwd,
    status,
    onSmithConfig,
}: SteeringInputProps) {
    const [message, setMessage] = useState("");
    const [sending, setSending] = useState(false);
    const [showAtMenu, setShowAtMenu] = useState(false);
    const [lastInjectedPtyId, setLastInjectedPtyId] = useState<string | null>(
        null,
    );
    const textareaRef = useRef<HTMLTextAreaElement>(null);

    const isSessionBusy = status === "thinking" || status === "executing_tool";

    // Listen for pty:inject events to auto-send the message via pty_write
    useEffect(() => {
        if (!lastInjectedPtyId) return;

        const unlisten = listen<string>(
            `pty:inject:${lastInjectedPtyId}`,
            async (event) => {
                const msg = event.payload;
                // Encode message as base64 and write to PTY
                const encoded = btoa(msg + "\n");
                try {
                    await invoke("pty_write", {
                        id: lastInjectedPtyId,
                        data: encoded,
                    });
                } catch (e) {
                    console.error("Failed to write steering message to PTY:", e);
                }
                setSending(false);
                setLastInjectedPtyId(null);
            },
        );

        return () => {
            unlisten.then((fn) => fn());
        };
    }, [lastInjectedPtyId]);

    const handleSend = useCallback(async () => {
        const trimmed = message.trim();
        if (!trimmed || sending) return;

        setSending(true);
        try {
            const ptyInfo = await invoke<{
                id: string;
                pid: number;
                cols: number;
                rows: number;
            }>("inject_session_message", {
                sessionId,
                message: trimmed,
                cwd,
            });
            setLastInjectedPtyId(ptyInfo.id);
            setMessage("");
        } catch (e) {
            console.error("Failed to inject message:", e);
            setSending(false);
        }
    }, [message, sending, sessionId, cwd]);

    const handleKeyDown = useCallback(
        (e: React.KeyboardEvent) => {
            if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleSend();
            }
            if (e.key === "@" && message.endsWith("")) {
                setShowAtMenu(true);
            }
            if (e.key === "Escape") {
                setShowAtMenu(false);
            }
        },
        [handleSend, message],
    );

    const handleAtSelect = useCallback(
        (prefix: string) => {
            setMessage((prev) => prev + prefix);
            setShowAtMenu(false);
            textareaRef.current?.focus();
        },
        [],
    );

    const handleFileAttach = useCallback(async () => {
        try {
            const selected = await open({
                multiple: true,
                directory: false,
            });
            if (selected) {
                const paths = Array.isArray(selected) ? selected : [selected];
                const refs = paths.map((p) => `@file:${p}`).join(" ");
                setMessage((prev) =>
                    prev ? `${prev} ${refs}` : refs,
                );
                textareaRef.current?.focus();
            }
        } catch {
            // User cancelled
        }
    }, []);

    // Auto-resize textarea
    const handleInput = useCallback(
        (e: React.ChangeEvent<HTMLTextAreaElement>) => {
            const val = e.target.value;
            setMessage(val);

            // Show @ menu when user types @
            if (val.endsWith("@")) {
                setShowAtMenu(true);
            } else {
                setShowAtMenu(false);
            }

            // Auto-resize
            const textarea = e.target;
            textarea.style.height = "auto";
            textarea.style.height =
                Math.min(textarea.scrollHeight, 120) + "px";
        },
        [],
    );

    return (
        <div className="shrink-0 border-t border-swarm-border bg-swarm-bg relative">
            {/* @ reference autocomplete popup */}
            {showAtMenu && (
                <div className="absolute bottom-full left-0 right-0 mb-1 mx-3 bg-swarm-surface border border-swarm-border rounded shadow-lg z-20">
                    <div className="py-1">
                        {AT_REFERENCES.map((ref) => (
                            <button
                                key={ref.prefix}
                                onClick={() => handleAtSelect(ref.prefix)}
                                className="w-full flex items-center gap-2 px-3 py-1.5 text-xs hover:bg-swarm-accent/10 transition-colors text-left"
                            >
                                <span className="text-sm">{ref.icon}</span>
                                <div>
                                    <span className="text-swarm-text font-medium">
                                        {ref.label}
                                    </span>
                                    <span className="text-swarm-text-dim ml-2">
                                        {ref.description}
                                    </span>
                                </div>
                            </button>
                        ))}
                    </div>
                </div>
            )}

            {/* Input area */}
            <div className="flex items-end gap-2 px-3 py-2">
                {/* File attach button */}
                <button
                    onClick={handleFileAttach}
                    className="shrink-0 w-7 h-7 flex items-center justify-center text-swarm-text-dim hover:text-swarm-text border border-swarm-border rounded hover:border-swarm-accent/30 transition-colors"
                    title="Attach files"
                >
                    <svg
                        className="w-3.5 h-3.5"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                    >
                        <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth={2}
                            d="M15.172 7l-6.586 6.586a2 2 0 102.828 2.828l6.414-6.586a4 4 0 00-5.656-5.656l-6.415 6.585a6 6 0 108.486 8.486L20.5 13"
                        />
                    </svg>
                </button>

                {/* Textarea */}
                <div className="flex-1 relative">
                    <textarea
                        ref={textareaRef}
                        value={message}
                        onChange={handleInput}
                        onKeyDown={handleKeyDown}
                        rows={1}
                        placeholder={
                            isSessionBusy
                                ? "Session busy - message will queue..."
                                : "Steer session... (@ for refs, Shift+Enter for newline)"
                        }
                        className="w-full bg-swarm-surface text-xs text-swarm-text px-3 py-1.5 rounded border border-swarm-border focus:border-swarm-accent/50 focus:outline-none resize-none placeholder:text-swarm-text-dim/50 leading-relaxed"
                        style={{ minHeight: "28px", maxHeight: "120px" }}
                        disabled={sending}
                    />
                </div>

                {/* Smith config button */}
                {onSmithConfig && (
                    <button
                        onClick={onSmithConfig}
                        className="shrink-0 w-7 h-7 flex items-center justify-center text-swarm-text-dim hover:text-swarm-text border border-swarm-border rounded hover:border-swarm-accent/30 transition-colors"
                        title="Smith override settings"
                    >
                        <svg
                            className="w-3.5 h-3.5"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                        >
                            <path
                                strokeLinecap="round"
                                strokeLinejoin="round"
                                strokeWidth={2}
                                d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                            />
                            <path
                                strokeLinecap="round"
                                strokeLinejoin="round"
                                strokeWidth={2}
                                d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                            />
                        </svg>
                    </button>
                )}

                {/* Send button */}
                <button
                    onClick={handleSend}
                    disabled={!message.trim() || sending}
                    className={`shrink-0 w-7 h-7 flex items-center justify-center rounded transition-colors ${
                        message.trim() && !sending
                            ? "bg-swarm-accent/20 text-swarm-accent border border-swarm-accent/30 hover:bg-swarm-accent/30"
                            : "text-swarm-text-dim/30 border border-swarm-border/50 cursor-not-allowed"
                    }`}
                    title="Send (Enter)"
                >
                    {sending ? (
                        <svg
                            className="w-3.5 h-3.5 animate-spin"
                            fill="none"
                            viewBox="0 0 24 24"
                        >
                            <circle
                                className="opacity-25"
                                cx="12"
                                cy="12"
                                r="10"
                                stroke="currentColor"
                                strokeWidth="4"
                            />
                            <path
                                className="opacity-75"
                                fill="currentColor"
                                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
                            />
                        </svg>
                    ) : (
                        <svg
                            className="w-3.5 h-3.5"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                        >
                            <path
                                strokeLinecap="round"
                                strokeLinejoin="round"
                                strokeWidth={2}
                                d="M12 19V5m0 0l-7 7m7-7l7 7"
                            />
                        </svg>
                    )}
                </button>
            </div>

            {/* Status indicator */}
            {isSessionBusy && (
                <div className="px-3 pb-1.5 -mt-0.5">
                    <span className="text-[10px] text-yellow-400/70">
                        Session is active - message will be delivered when idle
                    </span>
                </div>
            )}
        </div>
    );
}
