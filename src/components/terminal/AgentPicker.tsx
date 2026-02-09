import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect, useRef } from "react";

interface AgentDef {
    name: string;
    file_path: string;
    description: string | null;
}

interface AgentPickerProps {
    onSelect: (agentName: string) => void;
    onClose: () => void;
    /** Position anchor: "toolbar" for top-right, "inline" for in-place */
    position?: "toolbar" | "inline";
}

export function AgentPicker({
    onSelect,
    onClose,
    position = "toolbar",
}: AgentPickerProps) {
    const [agents, setAgents] = useState<AgentDef[]>([]);
    const [loading, setLoading] = useState(true);
    const ref = useRef<HTMLDivElement>(null);

    useEffect(() => {
        (async () => {
            try {
                const list = await invoke<AgentDef[]>("list_agents");
                setAgents(list);
            } catch (e) {
                console.error("Failed to list agents:", e);
                // Fallback to just claude
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

    // Close on click outside (use mouseup + RAF to avoid racing with the toggle button's click)
    useEffect(() => {
        let active = false;
        const id = requestAnimationFrame(() => {
            active = true;
        });
        const handler = (e: MouseEvent) => {
            if (
                active &&
                ref.current &&
                !ref.current.contains(e.target as Node)
            ) {
                onClose();
            }
        };
        document.addEventListener("mouseup", handler);
        return () => {
            cancelAnimationFrame(id);
            document.removeEventListener("mouseup", handler);
        };
    }, [onClose]);

    const posClass =
        position === "toolbar"
            ? "absolute right-0 top-full mt-1 z-50"
            : "absolute bottom-full left-0 mb-1 z-50";

    return (
        <div
            ref={ref}
            className={`${posClass} w-64 bg-swarm-surface border border-swarm-border rounded-lg shadow-xl overflow-hidden`}
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
