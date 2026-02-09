# Swarm-UI Bug Status - 2026-02-09

## What was attempted
Three bugs were addressed:
1. Open Terminal button does nothing
2. Diff only shows uncommitted changes (no commits, no untracked files)  
3. No back navigation from session detail to overview

## What was changed

### Files modified:
- `src/stores/layoutStore.ts` - setMode() now accepts keepPanes param
- `src/components/terminal/TerminalPane.tsx` - Fixed prop sync + conditional auto-spawn
- `src/components/layout/PaneGrid.tsx` - Back button, diff commits/untracked, terminal placeholder
- `src/components/session/SessionDetail.tsx` - Added onBack prop + back arrow button
- `src-tauri/src/commands/git.rs` - Added get_git_log, untracked files, untracked file diff
- `src-tauri/src/lib.rs` - Registered get_git_log command

### Open Terminal fix approach:
- TerminalPane now syncs ptyId prop via useEffect (was only using initial useState)
- Auto-spawn only fires when spawnConfig is provided
- List mode shows placeholder instead of auto-spawning
- handleOpenTerminal spawns PTY, sets pane terminalId, TerminalPane picks it up

## STATUS: USER REPORTS IT STILL FAILS
The user tested and says Open Terminal still doesn't work.
The app builds and runs without errors.
No JS console errors captured (WebKit inspector wasn't accessible).

## Next steps to debug:
1. Enable Tauri dev mode (npx tauri dev) to get live console output
2. Add console.log in handleOpenTerminal to trace the flow
3. Check if spawnTerminal invoke actually succeeds
4. Check if pane state update triggers re-render
5. Check if TerminalPane receives the new ptyId prop
6. Test with `npx tauri dev` not `npx tauri build` for faster iteration
