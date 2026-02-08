interface DiffViewerProps {
  oldContent: string;
  newContent: string;
  fileName: string;
}

/**
 * Inline diff viewer component.
 * TODO: Integrate react-diff-viewer-continued in Phase 8.
 */
export function DiffViewer({ oldContent: _oldContent, newContent, fileName }: DiffViewerProps) {
  return (
    <div className="p-4 font-mono text-sm">
      <div className="text-swarm-text-dim mb-2">{fileName}</div>
      <pre className="text-swarm-text whitespace-pre-wrap bg-swarm-bg p-3 rounded border border-swarm-border overflow-auto">
        {newContent || "No changes"}
      </pre>
    </div>
  );
}
