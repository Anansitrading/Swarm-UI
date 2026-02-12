import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect, useRef, useCallback } from "react";
import { createPortal } from "react-dom";

interface AgentDef {
    name: string;
    file_path: string;
    description: string | null;
}

interface AgentPickerProps {
    onSelect: (agentName: string) => void;
    onClose: () => void;
    /** Position anchor: "toolbar" for top-right, "inline" for centered */
    position?: "toolbar" | "inline";
    /** Optional anchor element ref for positioning near a button */
    anchorRef?: React.RefObject<HTMLElement | null>;
}

export function AgentPicker({
    onSelect,
    onClose,
    position = "toolbar",
    anchorRef,
}: AgentPickerProps) {
    const [agents, setAgents] = useState<AgentDef[]>([]);
    const [loading, setLoading] = useState(true);
    const ref = useRef<HTMLDivElement>(null);
    const [coords, setCoords] = useState<{ top: number; left: number } | null>(null);

    useEffect(() => {
        (async () => {
            try {
                const list = await invoke<AgentDef[]>("list_agents");
                setAgents(list);
            } catch (e) {
                console.error("Failed to list agents:", e);
                setAgents([
                    {
                        name: "claude",
                        file_path: "",
                        description: "Default Claude Code session",
                    },
                ]);
            }
            setLoading(false);
        })();
    }, []);

    // Position relative to anchor element
    useEffect(() => {
        if (anchorRef?.current) {
            const rect = anchorRef.current.getBoundingClientRect();
            if (position === "toolbar") {
                setCoords({ top: rect.bottom + 4, left: rect.right - 256 });
            } else {
                setCoords({ top: rect.bottom + 4, left: rect.left });
            }
        } else if (position === "toolbar") {
            setCoords({ top: 44, left: window.innerWidth - 264 });
        } else {
            // Center in viewport
            setCoords({ top: window.innerHeight / 2 - 100, left: window.innerWidth / 2 - 128 });
        }
    }, [anchorRef, position]);

    // Close on click outside
    const handleClickOutside = useCallback((e: MouseEvent) => {
        if (ref.current && !ref.current.contains(e.target as Node)) {
            onClose();
        }
    }, [onClose]);

    useEffect(() => {
        // Delay registering to avoid closing on the same click that opens
        const id = requestAnimationFrame(() => {
            document.addEventListener("mousedown", handleClickOutside);
        });
        return () => {
            cancelAnimationFrame(id);
            document.removeEventListener("mousedown", handleClickOutside);
        };
    }, [handleClickOutside]);

    // Close on Escape
    useEffect(() => {
        const handler = (e: KeyboardEvent) => {
            if (e.key === "Escape") onClose();
        };
        document.addEventListener("keydown", handler);
        return () => document.removeEventListener("keydown", handler);
    }, [onClose]);

    if (!coords) return null;

    const picker = (
        <div
            ref={ref}
            className="fixed z-[9999] w-64 bg-swarm-surface border border-swarm-border rounded-lg shadow-xl overflow-hidden"
            style={{ top: coords.top, left: Math.max(8, coords.left) }}
        >
            <div className="px-3 py-1.5 border-b border-swarm-border bg-swarm-bg">
                <span className="text-[10px] text-swarm-text-dim uppercase tracking-wide">
                    Launch Agent Session
                </span>
            </div>
            {loading ? (
                <div className="flex items-center justify-center py-4">
                    <div className="animate-spin h-4 w-4 border-2 border-swarm-accent border-t-transparent rounded-full" />
                </div>
            ) : (
                <div className="py-1 max-h-64 overflow-y-auto">
                    {agents.map((agent) => (
                        <button
                            key={agent.name}
                            onClick={() => onSelect(agent.name)}
                            className="w-full text-left px-3 py-2 hover:bg-swarm-accent/10 transition-colors"
                        >
                            <div className="flex items-center gap-2">
                                <AgentIcon name={agent.name} />
                                <span className="text-xs font-medium text-swarm-text">
                                    {agent.name}
                                </span>
                            </div>
                            {agent.description && (
                                <div className="text-[10px] text-swarm-text-dim mt-0.5 pl-6 truncate">
                                    {agent.description}
                                </div>
                            )}
                        </button>
                    ))}
                </div>
            )}
        </div>
    );

    return createPortal(picker, document.body);
}

function AgentIcon({ name }: { name: string }) {
    const colors: Record<string, string> = {
        claude: "bg-blue-400",
        oracle: "bg-purple-400",
        smith: "bg-red-400",
        trinity: "bg-green-400",
    };
    const color = colors[name] || "bg-gray-400";
    const letter = name.charAt(0).toUpperCase();

    return (
        <span
            className={`w-4 h-4 rounded flex items-center justify-center text-[9px] font-bold text-white ${color}`}
        >
            {letter}
        </span>
    );
}
