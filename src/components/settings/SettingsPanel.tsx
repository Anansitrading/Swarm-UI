import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "../../stores/settingsStore";

export function SettingsPanel() {
    const {
        settings,
        connectionStatus,
        testing,
        saveSettings,
        loadSettings,
        loaded,
    } = useSettingsStore();
    const [localSettings, setLocalSettings] = useState(settings);
    const [showToken, setShowToken] = useState(false);
    const [saved, setSaved] = useState(false);

    useEffect(() => {
        if (!loaded) loadSettings();
    }, [loaded, loadSettings]);

    useEffect(() => {
        setLocalSettings(settings);
    }, [settings]);

    const handleSave = async () => {
        await saveSettings(localSettings);
        setSaved(true);
        setTimeout(() => setSaved(false), 2000);
    };

    // Test connection uses the current LOCAL (possibly unsaved) settings
    const handleTestConnection = async () => {
        useSettingsStore.setState({ testing: true, connectionStatus: null });
        try {
            // Configure backend with the current local values, then test
            const result = await invoke<string>("sprite_configure", {
                baseUrl: localSettings.spriteApiUrl,
                token: localSettings.spriteApiToken,
            });
            useSettingsStore.setState({
                connectionStatus: result,
                testing: false,
            });
        } catch (e) {
            useSettingsStore.setState({
                connectionStatus: `Error: ${e}`,
                testing: false,
            });
        }
    };

    const hasChanges =
        localSettings.spriteApiUrl !== settings.spriteApiUrl ||
        localSettings.spriteApiToken !== settings.spriteApiToken ||
        localSettings.spriteOrg !== settings.spriteOrg ||
        localSettings.terminalFont !== settings.terminalFont ||
        localSettings.terminalFontSize !== settings.terminalFontSize;

    return (
        <div className="flex flex-col h-full bg-swarm-surface overflow-y-auto">
            <div className="border-b border-swarm-border px-4 py-3">
                <h2 className="text-sm font-medium text-swarm-text">
                    Settings
                </h2>
                <p className="text-[10px] text-swarm-text-dim mt-0.5">
                    Configure Sprites API, terminal preferences, and more.
                </p>
            </div>

            <div className="flex-1 p-4 space-y-6">
                {/* Sprites API Section */}
                <section>
                    <h3 className="text-xs font-medium text-swarm-text mb-3 uppercase tracking-wide">
                        Sprites API
                    </h3>
                    <div className="space-y-3">
                        <Field
                            label="API URL"
                            value={localSettings.spriteApiUrl}
                            onChange={(v) =>
                                setLocalSettings({
                                    ...localSettings,
                                    spriteApiUrl: v,
                                })
                            }
                            placeholder="https://api.sprites.dev"
                        />
                        <div>
                            <label className="block text-[11px] text-swarm-text-dim mb-1">
                                API Token
                            </label>
                            <div className="flex gap-2">
                                <input
                                    type={showToken ? "text" : "password"}
                                    value={localSettings.spriteApiToken}
                                    onChange={(e) =>
                                        setLocalSettings({
                                            ...localSettings,
                                            spriteApiToken: e.target.value,
                                        })
                                    }
                                    placeholder="your-org/user-id/client-id/token"
                                    className="flex-1 px-2 py-1.5 text-xs bg-swarm-bg border border-swarm-border rounded text-swarm-text placeholder-swarm-text-dim/50 focus:border-swarm-accent/50 focus:outline-none font-mono"
                                />
                                <button
                                    onClick={() => setShowToken(!showToken)}
                                    className="px-2 py-1.5 text-[10px] text-swarm-text-dim border border-swarm-border rounded hover:text-swarm-text hover:border-swarm-accent/30 transition-colors"
                                >
                                    {showToken ? "Hide" : "Show"}
                                </button>
                            </div>
                        </div>
                        <Field
                            label="Organization"
                            value={localSettings.spriteOrg}
                            onChange={(v) =>
                                setLocalSettings({
                                    ...localSettings,
                                    spriteOrg: v,
                                })
                            }
                            placeholder="david-simpson"
                        />

                        {/* Connection test */}
                        <div className="flex items-center gap-2 pt-1">
                            <button
                                onClick={handleTestConnection}
                                disabled={
                                    testing || !localSettings.spriteApiToken
                                }
                                className="px-3 py-1.5 text-xs bg-swarm-accent/20 text-swarm-accent border border-swarm-accent/30 rounded hover:bg-swarm-accent/30 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                            >
                                {testing ? "Testing..." : "Test Connection"}
                            </button>
                            {connectionStatus && (
                                <span
                                    className={`text-[10px] ${
                                        connectionStatus.startsWith("Error")
                                            ? "text-red-400"
                                            : "text-green-400"
                                    }`}
                                >
                                    {connectionStatus}
                                </span>
                            )}
                        </div>
                    </div>
                </section>

                {/* Terminal Section */}
                <section>
                    <h3 className="text-xs font-medium text-swarm-text mb-3 uppercase tracking-wide">
                        Terminal
                    </h3>
                    <div className="space-y-3">
                        <Field
                            label="Font Family"
                            value={localSettings.terminalFont}
                            onChange={(v) =>
                                setLocalSettings({
                                    ...localSettings,
                                    terminalFont: v,
                                })
                            }
                            placeholder="JetBrains Mono"
                        />
                        <div>
                            <label className="block text-[11px] text-swarm-text-dim mb-1">
                                Font Size
                            </label>
                            <input
                                type="number"
                                min={8}
                                max={24}
                                value={localSettings.terminalFontSize}
                                onChange={(e) =>
                                    setLocalSettings({
                                        ...localSettings,
                                        terminalFontSize:
                                            parseInt(e.target.value) || 13,
                                    })
                                }
                                className="w-20 px-2 py-1.5 text-xs bg-swarm-bg border border-swarm-border rounded text-swarm-text focus:border-swarm-accent/50 focus:outline-none"
                            />
                        </div>
                    </div>
                </section>
            </div>

            {/* Save bar */}
            <div className="shrink-0 flex items-center justify-between px-4 py-3 border-t border-swarm-border bg-swarm-bg">
                <div className="text-[10px] text-swarm-text-dim">
                    {saved && (
                        <span className="text-green-400">Settings saved!</span>
                    )}
                    {hasChanges && !saved && <span>Unsaved changes</span>}
                </div>
                <button
                    onClick={handleSave}
                    disabled={!hasChanges}
                    className="px-4 py-1.5 text-xs bg-swarm-accent text-white rounded hover:bg-swarm-accent-dim transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                >
                    Save
                </button>
            </div>
        </div>
    );
}

function Field({
    label,
    value,
    onChange,
    placeholder,
}: {
    label: string;
    value: string;
    onChange: (v: string) => void;
    placeholder?: string;
}) {
    return (
        <div>
            <label className="block text-[11px] text-swarm-text-dim mb-1">
                {label}
            </label>
            <input
                type="text"
                value={value}
                onChange={(e) => onChange(e.target.value)}
                placeholder={placeholder}
                className="w-full px-2 py-1.5 text-xs bg-swarm-bg border border-swarm-border rounded text-swarm-text placeholder-swarm-text-dim/50 focus:border-swarm-accent/50 focus:outline-none"
            />
        </div>
    );
}
