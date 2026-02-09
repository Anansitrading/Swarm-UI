import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect, useCallback } from "react";

interface SmithPanelProps {
    sessionId: string;
    onClose: () => void;
}

interface SmithOverride {
    enabled: boolean;
    instructions: string;
}

export function SmithPanel({ sessionId, onClose }: SmithPanelProps) {
    const [enabled, setEnabled] = useState(false);
    const [instructions, setInstructions] = useState("");
    const [saving, setSaving] = useState(false);
    const [saved, setSaved] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Load existing override on mount
    useEffect(() => {
        (async () => {
            try {
                const data = await invoke<SmithOverride>(
                    "load_smith_override",
                    {
                        sessionId,
                    },
                );
                setEnabled(data.enabled);
                setInstructions(data.instructions || "");
            } catch {
                // No override yet - defaults are fine
            }
        })();
    }, [sessionId]);

    const handleSave = useCallback(async () => {
        setSaving(true);
        setError(null);
        try {
            await invoke("save_smith_override", {
                sessionId,
                enabled,
                instructions,
            });
            setSaved(true);
            setTimeout(() => setSaved(false), 2000);
        } catch (e) {
            console.error("Failed to save Smith override:", e);
            setError(String(e));
        }
        setSaving(false);
    }, [sessionId, enabled, instructions]);

    return (
        <div className="shrink-0 border-t border-swarm-border bg-swarm-surface">
            <div className="px-3 py-2">
                {/* Header */}
                <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                        <span className="text-xs font-medium text-swarm-text">
                            Smith Override
                        </span>
                        <label className="flex items-center gap-1 cursor-pointer">
                            <input
                                type="checkbox"
                                checked={enabled}
                                onChange={(e) => setEnabled(e.target.checked)}
                                className="w-3 h-3 accent-swarm-accent"
                            />
                            <span className="text-[10px] text-swarm-text-dim">
                                {enabled ? "Active" : "Off"}
                            </span>
                        </label>
                    </div>
                    <button
                        onClick={onClose}
                        className="text-swarm-text-dim hover:text-swarm-text text-xs"
                    >
                        &times;
                    </button>
                </div>

                {/* Instructions textarea */}
                <textarea
                    value={instructions}
                    onChange={(e) => setInstructions(e.target.value)}
                    placeholder="Custom Smith instructions for this session..."
                    rows={3}
                    className="w-full bg-swarm-bg text-xs text-swarm-text px-2 py-1.5 rounded border border-swarm-border focus:border-swarm-accent/50 focus:outline-none resize-none placeholder:text-swarm-text-dim/50"
                />

                {/* Error display */}
                {error && (
                    <div className="text-[10px] text-red-400 mt-1">
                        Save failed: {error}
                    </div>
                )}

                {/* Save button */}
                <div className="flex items-center justify-between mt-1.5">
                    <span className="text-[10px] text-swarm-text-dim">
                        Overrides smith-pre-tool-use hook for this session
                    </span>
                    <button
                        onClick={handleSave}
                        disabled={saving}
                        className={`px-2 py-0.5 text-[10px] rounded border transition-colors ${
                            saved
                                ? "text-green-400 border-green-400/30 bg-green-400/10"
                                : "text-swarm-accent border-swarm-accent/30 bg-swarm-accent/10 hover:bg-swarm-accent/20"
                        }`}
                    >
                        {saving ? "Saving..." : saved ? "Saved" : "Save"}
                    </button>
                </div>
            </div>
        </div>
    );
}
