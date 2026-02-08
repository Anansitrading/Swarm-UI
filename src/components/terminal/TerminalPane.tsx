import { useRef, useEffect, useState } from "react";
import { useTerminal } from "../../hooks/useTerminal";
import { useTerminalStore } from "../../stores/terminalStore";
import type { PtySpawnConfig } from "../../types/terminal";

interface TerminalPaneProps {
  ptyId?: string;
  spawnConfig?: PtySpawnConfig;
  className?: string;
}

export function TerminalPane({ ptyId, spawnConfig, className }: TerminalPaneProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [currentPtyId, setCurrentPtyId] = useState(ptyId);
  const { spawnTerminal } = useTerminalStore();

  // Auto-spawn a PTY if none provided
  useEffect(() => {
    if (!currentPtyId) {
      spawnTerminal(spawnConfig).then((info) => {
        setCurrentPtyId(info.id);
      });
    }
  }, [currentPtyId, spawnConfig, spawnTerminal]);

  // Only initialize terminal when we have a PTY ID
  if (currentPtyId) {
    return (
      <TerminalView
        ptyId={currentPtyId}
        containerRef={containerRef}
        className={className}
      />
    );
  }

  return (
    <div className={`flex items-center justify-center bg-swarm-bg ${className ?? ""}`}>
      <span className="text-swarm-text-dim text-sm">Starting terminal...</span>
    </div>
  );
}

function TerminalView({
  ptyId,
  containerRef,
  className,
}: {
  ptyId: string;
  containerRef: React.RefObject<HTMLDivElement | null>;
  className?: string;
}) {
  useTerminal({ ptyId, containerRef });

  return (
    <div
      ref={containerRef}
      className={`w-full h-full min-h-0 bg-swarm-bg ${className ?? ""}`}
    />
  );
}
