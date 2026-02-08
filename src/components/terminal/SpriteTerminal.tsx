import { TerminalPane } from "./TerminalPane";

interface SpriteTerminalProps {
  spriteName: string;
  className?: string;
}

/**
 * Sprite terminal that spawns `sprite console -s {name}` as a PTY child.
 * The sprite CLI handles WebSocket connections internally.
 */
export function SpriteTerminal({ spriteName, className }: SpriteTerminalProps) {
  return (
    <TerminalPane
      spawnConfig={{
        shell: "sprite",
        args: ["console", "-s", spriteName],
      }}
      className={className}
    />
  );
}
