import { useLayoutStore } from "../../stores/layoutStore";
import type { LayoutMode } from "../../types/terminal";

const LAYOUT_OPTIONS: { mode: LayoutMode; icon: string; label: string }[] = [
  { mode: "single", icon: "□", label: "Single" },
  { mode: "list", icon: "▌□", label: "List" },
  { mode: "two_column", icon: "□□", label: "Two" },
  { mode: "three_column", icon: "□□□", label: "Three" },
  { mode: "sprite_grid", icon: "⊞", label: "Grid" },
];

export function Toolbar() {
  const { mode, setMode, toggleSidebar, sidebarCollapsed } = useLayoutStore();

  return (
    <div className="flex items-center justify-between h-10 px-3 bg-swarm-surface border-b border-swarm-border">
      <div className="flex items-center gap-2">
        <button
          onClick={toggleSidebar}
          className="text-swarm-text-dim hover:text-swarm-text p-1 rounded transition-colors"
          title={sidebarCollapsed ? "Show sidebar" : "Hide sidebar"}
        >
          <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5" />
          </svg>
        </button>
        <span className="text-sm font-semibold text-swarm-accent">Swarm-UI</span>
      </div>

      <div className="flex items-center gap-1 bg-swarm-bg rounded-lg p-0.5">
        {LAYOUT_OPTIONS.map((opt) => (
          <button
            key={opt.mode}
            onClick={() => setMode(opt.mode)}
            className={`px-2 py-1 rounded text-xs font-mono transition-colors ${
              mode === opt.mode
                ? "bg-swarm-accent text-white"
                : "text-swarm-text-dim hover:text-swarm-text"
            }`}
            title={opt.label}
          >
            {opt.icon}
          </button>
        ))}
      </div>

      <div className="w-16" /> {/* Spacer for balance */}
    </div>
  );
}
