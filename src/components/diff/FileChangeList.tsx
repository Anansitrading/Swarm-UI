import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DiffViewer } from "./DiffViewer";

export interface FileChange {
  path: string;
  toolName: string;
  oldContent: string;
  newContent: string;
  timestamp: number;
}

interface FileChangeListProps {
  changes: FileChange[];
}

export function FileChangeList({ changes }: FileChangeListProps) {
  const [selectedIdx, setSelectedIdx] = useState<number | null>(null);
  const [fileContent, setFileContent] = useState<string | null>(null);

  const handleSelect = useCallback(
    async (idx: number) => {
      setSelectedIdx(idx);
      const change = changes[idx];
      // Try to read current file content for comparison
      if (change.oldContent === "" && change.path) {
        try {
          const content = await invoke<string>("read_file", { path: change.path });
          setFileContent(content);
        } catch {
          setFileContent(null);
        }
      }
    },
    [changes]
  );

  if (changes.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-swarm-text-dim text-sm">
        No file changes in this session yet.
      </div>
    );
  }

  const selectedChange = selectedIdx !== null ? changes[selectedIdx] : null;

  return (
    <div className="flex flex-col h-full">
      {/* File list */}
      <div className="border-b border-swarm-border overflow-y-auto max-h-48">
        {changes.map((change, idx) => {
          const fileName = change.path.split("/").pop() || change.path;
          return (
            <button
              key={`${change.path}-${change.timestamp}`}
              onClick={() => handleSelect(idx)}
              className={`w-full text-left px-3 py-1.5 text-xs border-b border-swarm-border/50 transition-colors ${
                selectedIdx === idx
                  ? "bg-swarm-accent/10 text-swarm-accent"
                  : "text-swarm-text hover:bg-swarm-surface"
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="font-mono truncate">{fileName}</span>
                <span className="text-swarm-text-dim shrink-0 ml-2">
                  {change.toolName === "Write" ? "+" : "~"}
                </span>
              </div>
            </button>
          );
        })}
      </div>

      {/* Diff view */}
      <div className="flex-1 min-h-0">
        {selectedChange ? (
          <DiffViewer
            oldContent={selectedChange.oldContent || fileContent || ""}
            newContent={selectedChange.newContent}
            fileName={selectedChange.path}
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
