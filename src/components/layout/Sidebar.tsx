import { useLayoutStore } from "../../stores/layoutStore";
import { SessionList } from "../session/SessionList";
import { SpriteList } from "../sprite/SpriteList";
import { TeamList } from "../team/TeamList";
import { SettingsPanel } from "../settings/SettingsPanel";

interface SidebarProps {
    onSpriteSelect: (name: string) => void;
}

const TABS = [
    { key: "sessions" as const, label: "Sessions" },
    { key: "teams" as const, label: "Teams" },
    { key: "sprites" as const, label: "Sprites" },
    { key: "settings" as const, label: "Settings" },
];

export function Sidebar({ onSpriteSelect }: SidebarProps) {
    const { sidebarTab, setSidebarTab, sidebarCollapsed } = useLayoutStore();

    if (sidebarCollapsed) return null;

    return (
        <div className="w-72 flex flex-col bg-swarm-surface border-r border-swarm-border overflow-hidden">
            {/* Tab bar */}
            <div className="flex border-b border-swarm-border">
                {TABS.map((tab) => (
                    <button
                        key={tab.key}
                        onClick={() => setSidebarTab(tab.key)}
                        className={`flex-1 px-2 py-2 text-xs font-medium transition-colors ${
                            sidebarTab === tab.key
                                ? "text-swarm-accent border-b-2 border-swarm-accent"
                                : "text-swarm-text-dim hover:text-swarm-text"
                        }`}
                    >
                        {tab.label}
                    </button>
                ))}
            </div>

            {/* Tab content */}
            <div className="flex-1 overflow-y-auto">
                {sidebarTab === "sessions" && <SessionList />}
                {sidebarTab === "teams" && <TeamList />}
                {sidebarTab === "sprites" && (
                    <SpriteList onSelect={onSpriteSelect} />
                )}
                {sidebarTab === "settings" && <SettingsPanel />}
            </div>
        </div>
    );
}
