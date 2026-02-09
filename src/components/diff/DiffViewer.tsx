import ReactDiffViewer, { DiffMethod } from "react-diff-viewer-continued";

interface DiffViewerProps {
  oldContent: string;
  newContent: string;
  fileName: string;
}

const darkStyles = {
  variables: {
    dark: {
      diffViewerBackground: "#0a0a0f",
      diffViewerTitleBackground: "#12121a",
      diffViewerTitleColor: "#e2e8f0",
      diffViewerTitleBorderColor: "#1e293b",
      addedBackground: "#132b1f",
      addedColor: "#4ade80",
      removedBackground: "#2b1313",
      removedColor: "#f87171",
      wordAddedBackground: "#166534",
      wordRemovedBackground: "#991b1b",
      addedGutterBackground: "#0d2818",
      removedGutterBackground: "#1f0d0d",
      gutterBackground: "#0f0f17",
      gutterBackgroundDark: "#0a0a0f",
      highlightBackground: "#7c3aed20",
      highlightGutterBackground: "#7c3aed10",
      codeFoldGutterBackground: "#1e1e2e",
      codeFoldBackground: "#12121a",
      emptyLineBackground: "#0f0f17",
      gutterColor: "#64748b",
      addedGutterColor: "#22c55e",
      removedGutterColor: "#ef4444",
      codeFoldContentColor: "#64748b",
      diffViewerColor: "#e2e8f0",
    },
  },
};

export function DiffViewer({ oldContent, newContent, fileName }: DiffViewerProps) {
  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center px-3 py-2 bg-swarm-surface border-b border-swarm-border">
        <span className="text-xs font-mono text-swarm-text">{fileName}</span>
      </div>
      <div className="flex-1 overflow-auto text-xs">
        <ReactDiffViewer
          oldValue={oldContent}
          newValue={newContent}
          splitView={false}
          useDarkTheme={true}
          styles={darkStyles}
          compareMethod={DiffMethod.WORDS}
          hideLineNumbers={false}
        />
      </div>
    </div>
  );
}
