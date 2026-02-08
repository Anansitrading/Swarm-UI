export interface PtyInfo {
  id: string;
  pid: number;
  cols: number;
  rows: number;
}

export interface PtySpawnConfig {
  shell?: string;
  args?: string[];
  cwd?: string;
  env?: Record<string, string>;
  cols?: number;
  rows?: number;
}

export type LayoutMode =
  | "single"
  | "list"
  | "two_column"
  | "three_column"
  | "sprite_grid";

export interface PaneConfig {
  id: string;
  type: "terminal" | "session" | "sprite" | "diff" | "empty";
  terminalId?: string;
  sessionId?: string;
  spriteName?: string;
}
