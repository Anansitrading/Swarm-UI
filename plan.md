# Swarm-UI - Linux AgentHub with Sprite Swarm Integration

## Context

AgentHub is a macOS-only Swift/SwiftUI app that monitors Claude Code sessions with embedded terminals, multi-pane layouts, JSONL parsing, and git worktree detection. We need a Linux equivalent that replicates all features AND adds sprites.dev swarm integration (live terminal grid for 10+ sprite VMs, 20-bot pool visualization).

**Technology choice: Tauri 2.0 + React + TypeScript + xterm.js** (see research justification in conversation). Zig eliminated (no ecosystem). Pure Rust possible but fragmented (2-3x slower to build). Tauri gives us xterm.js terminals + CSS Grid layouts + Rust backend performance.

**Repo**: `Anansitrading/Swarm-UI` on GitHub
**Project driver**: [get-shit-done](https://github.com/glittercowboy/get-shit-done.git) task runner

## Architecture Overview

```
┌─────────────────────────────────────────────┐
│              Tauri 2.0 Shell                 │
│  ┌────────────────────────────────────────┐  │
│  │     React 19 + TypeScript Frontend     │  │
│  │  xterm.js terminals (WebGL renderer)   │  │
│  │  CSS Grid multi-pane layouts           │  │
│  │  Zustand state management              │  │
│  │  Tailwind CSS styling                  │  │
│  └────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────┐  │
│  │         Rust Backend (Tauri)           │  │
│  │  portable-pty (PTY management)         │  │
│  │  notify (inotify file watching)        │  │
│  │  JSONL incremental parser              │  │
│  │  Sprite CLI subprocess wrapper         │  │
│  │  Process PID registry                  │  │
│  │  Git worktree detection                │  │
│  └────────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

## Directory Structure

```
/home/devuser/Swarm-UI/
├── src-tauri/                    # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs               # Entry point
│       ├── lib.rs                # Plugin + command registration
│       ├── state.rs              # Shared AppState (Mutex-wrapped)
│       ├── error.rs              # Error types
│       ├── commands/
│       │   ├── pty.rs            # pty_spawn, pty_write, pty_resize, pty_kill
│       │   ├── session.rs        # list_sessions, get_session_detail, start_watcher
│       │   ├── process.rs        # find_claude_processes, kill_process
│       │   ├── sprite.rs         # sprite_list, sprite_exec, sprite_console_attach
│       │   ├── git.rs            # detect_worktree, get_git_branch
│       │   └── filesystem.rs     # read_file (for diff viewer)
│       ├── watchers/
│       │   ├── jsonl_watcher.rs  # inotify on ~/.claude/projects/**/*.jsonl
│       │   └── pool_watcher.rs   # inotify on ~/.cortex/sprite-pool.json
│       ├── parsers/
│       │   ├── jsonl_parser.rs   # Incremental JSONL parser (port from Swift)
│       │   └── session_types.rs  # Rust session/entry data models
│       └── sprite/
│           ├── cli.rs            # sprite CLI subprocess wrapper
│           └── pool.rs           # Parse sprite-pool.json
├── src/                          # React frontend
│   ├── main.tsx
│   ├── App.tsx
│   ├── stores/
│   │   ├── sessionStore.ts       # Sessions, status, context usage
│   │   ├── terminalStore.ts      # Terminal instances lifecycle
│   │   ├── spriteStore.ts        # Sprites, pool state
│   │   └── layoutStore.ts        # Layout mode, pane config
│   ├── components/
│   │   ├── layout/
│   │   │   ├── AppShell.tsx      # Top-level: titlebar, sidebar, panes
│   │   │   ├── Sidebar.tsx       # Tabs: Sessions | Sprites | Bot Pool
│   │   │   ├── PaneGrid.tsx      # CSS Grid for 4 layout modes
│   │   │   └── Toolbar.tsx       # Layout toggle, actions
│   │   ├── terminal/
│   │   │   ├── TerminalPane.tsx  # xterm.js wrapper + PTY IPC
│   │   │   └── SpriteTerminal.tsx # Sprite console via PTY-wrapped CLI
│   │   ├── session/
│   │   │   ├── SessionList.tsx
│   │   │   ├── SessionCard.tsx
│   │   │   ├── StatusBadge.tsx   # Thinking/Executing/Idle badges
│   │   │   └── ContextBar.tsx    # Context window usage bar
│   │   ├── sprite/
│   │   │   ├── SpriteList.tsx
│   │   │   ├── SpriteGrid.tsx    # Multi-sprite terminal grid
│   │   │   ├── BotPoolView.tsx   # 4x5 bot pool visualization
│   │   │   └── BotPoolCard.tsx   # Single bot slot card
│   │   └── diff/
│   │       └── DiffViewer.tsx    # Inline diff viewer
│   ├── hooks/
│   │   ├── useTerminal.ts        # xterm.js instance management
│   │   ├── useSession.ts         # Session data subscription
│   │   └── useLayout.ts          # Layout mode switching
│   └── types/
│       ├── session.ts            # Session, SessionStatus, ContextUsage
│       ├── sprite.ts             # Sprite, BotSlot, SpritePool
│       └── terminal.ts           # TerminalInstance, LayoutMode
├── package.json
├── vite.config.ts
└── tailwind.config.js
```

## Data Flow

### Terminal: User keystrokes -> xterm.js -> invoke("pty_write") -> Rust portable-pty -> child process -> PTY read loop -> emit("pty:data:{id}") -> xterm.js.write()

### Session monitoring: inotify on ~/.claude/projects/ -> read new JSONL lines from offset -> parse entries -> detect status -> emit("session:updated") -> Zustand store -> React re-render

### Sprite console: spawn `sprite console -s {name}` as PTY child -> same data flow as local terminal (PTY-wrapped CLI handles WebSocket internally)

### Bot pool: inotify on ~/.cortex/sprite-pool.json -> parse 20-slot pool -> emit("pool:updated") -> Zustand store -> BotPoolView (4x5 grid)

## Layout Modes (matching AgentHub)

| Mode | CSS Grid | Description |
|------|----------|-------------|
| single | `1fr` | One full-width pane |
| list | `300px 1fr` | Sidebar list + detail |
| two_column | `1fr 1fr` | Two equal panes |
| three_column | `1fr 1fr 1fr` | Three equal panes |
| sprite_grid | `repeat(auto-fill, minmax(400px, 1fr))` | N-pane sprite grid (NEW) |

## Key Tauri Commands (Rust backend API)

```
PTY:      pty_spawn, pty_write, pty_resize, pty_kill
Sessions: list_sessions, get_session_detail, start_session_watcher
Process:  find_claude_processes, kill_process
Sprites:  sprite_list, sprite_exec, sprite_console_attach, sprite_checkpoint_*
Git:      detect_worktree, get_git_branch
Pool:     get_bot_pool_state, start_pool_watcher
Files:    read_file, read_file_range
```

## Implementation Phases

### Phase 0: Repo Bootstrap + Tooling
- Create GitHub repo `Anansitrading/Swarm-UI` via `gh repo create`
- Push this plan as `plan.md` as first commit
- Clone and install `get-shit-done` from https://github.com/glittercowboy/get-shit-done.git
- Fetch relevant docs via Exa (Tauri 2.0 API, xterm.js, portable-pty, notify crate) and Context7
- Configure get-shit-done to orchestrate the remaining phases
- **Milestone**: Repo live, tooling ready, docs cached

### Phase 1: Scaffold + PTY Foundation (2-3 days)
- `npm create tauri-app@latest swarmhub` (React + TypeScript template)
- Add Rust deps: portable-pty, notify, serde, serde_json, tokio, tracing
- Add npm deps: xterm, @xterm/addon-webgl, @xterm/addon-fit, zustand, tailwindcss
- Implement `commands/pty.rs` (spawn, write, resize, kill)
- Implement `TerminalPane.tsx` with xterm.js WebGL
- Basic `AppShell.tsx` with single pane
- **Milestone**: Working terminal in Tauri window

### Phase 2: JSONL Session Monitoring (3-4 days) [depends: Phase 1]
- Port Swift `SessionJSONLParser` to Rust `parsers/jsonl_parser.rs`
- Implement incremental reading (track file offset, read only new lines)
- Implement `watchers/jsonl_watcher.rs` using notify crate (inotify)
- Implement status detection: thinking/executing_tool/awaiting_approval/waiting/idle
- Implement token usage aggregation from message.usage fields
- Build SessionList, SessionCard, StatusBadge, ContextBar components
- Handle path decoding (`-home-devuser-Kijko-MVP` -> `/home/devuser/Kijko-MVP`)
- **Milestone**: Live session list with status badges and context bars

### Phase 3: Multi-Pane Layout (2 days) [depends: Phase 1]
- Implement PaneGrid.tsx with CSS Grid for all 5 modes
- Implement layoutStore.ts with mode switching
- Toolbar with layout toggle buttons (icons matching AgentHub)
- Collapsible sidebar
- **Milestone**: Toggle between all layout modes, assign content to panes

### Phase 4: Process Management (1-2 days) [depends: Phase 2]
- Scan /proc for Claude processes, link to sessions by CWD
- Kill buttons (SIGTERM/SIGKILL) on session cards
- Process cleanup on app exit
- **Milestone**: Session cards show PIDs, kill works

### Phase 5: Git Integration (1 day) [depends: Phase 2]
- `git worktree list` detection from Rust
- Branch labels on session cards
- Session-to-worktree matching
- **Milestone**: Sessions show git branches

### Phase 6: Sprite Integration (3-4 days) [depends: Phase 1, Phase 3]
- `sprite/cli.rs`: subprocess wrapper for sprite binary
- Sprite commands: list, exec, console_attach, checkpoints
- SpriteList, SpriteCard, SpriteTerminal components
- SpriteGrid: multi-sprite terminal view (auto-fill grid)
- Checkpoint create/restore UI
- **Milestone**: Attach to sprite consoles, manage checkpoints

### Phase 7: Bot Pool Visualization (2 days) [depends: Phase 6]
- Parse ~/.cortex/sprite-pool.json (20-slot schema)
- inotify watcher for pool state changes
- BotPoolView: 4x5 grid with status colors
- BotPoolCard: bot number, ticket, role, heartbeat
- Click-to-focus: card click opens sprite terminal
- **Milestone**: Live 20-bot pool dashboard

### Phase 8: Diff Viewer (2 days) [depends: Phase 2, Phase 3]
- Extract file changes from JSONL tool_use entries (Write/Edit tools)
- react-diff-viewer-continued for inline diff display
- Accept/reject controls
- **Milestone**: File change diffs shown in panes

### Phase 9: Polish (2-3 days) [depends: all]
- Dark theme (matching Oracle/Panopticon aesthetic)
- Keyboard shortcuts (Ctrl+T, Ctrl+1/2/3, Escape)
- Error boundaries, loading skeletons
- Performance: <16ms frame time with WebGL
- Rust tests: parsers, watchers, commands
- Frontend tests: stores, status detection
- **Milestone**: Production-ready

## Critical Path

```
Phase 1 ─┬─> Phase 2 ─┬─> Phase 4 (Process)
          │            ├─> Phase 5 (Git)
          │            └─> Phase 8 (Diff)
          └─> Phase 3 ─┬─> Phase 6 (Sprites) ──> Phase 7 (Bot Pool)
                       └─────────────────────────────────────────> Phase 9
```

**Total: ~18-23 days single developer. Phases 2+3 parallelizable.**

## Reference Files

| File | Purpose |
|------|---------|
| `/home/devuser/sprite-mcp-server/src/index.ts` | Sprite CLI wrappers, output parsing patterns |
| `/home/devuser/Oracle-Cortex/scripts/smith/sprite_pool_manager.py` | 20-bot pool schema, claim/release lifecycle |
| `/home/devuser/.cortex/sprite-pool.json` | Live pool state (exact schema to parse) |
| `/home/devuser/panopticon/ui/src-tauri/Cargo.toml` | Existing Tauri 2.0 project with proven deps |
| `/tmp/AgentHub/app/modules/AgentHubCore/Sources/AgentHub/Services/SessionJSONLParser.swift` | JSONL parser to port |
| `/tmp/AgentHub/app/modules/AgentHubCore/Sources/AgentHub/UI/MonitoringPanelView.swift` | Layout mode reference |

## Verification

1. **Terminal**: Open app -> new terminal pane -> type `ls` -> see output
2. **Sessions**: Start Claude Code session -> see it appear in sidebar with status badge
3. **Layout**: Toggle all 5 modes -> verify grid changes correctly
4. **Sprites**: Click sprite in sidebar -> console opens in pane -> interactive shell works
5. **Bot Pool**: Modify sprite-pool.json externally -> verify BotPoolView updates within 2s
6. **Diff**: Claude edits a file -> diff appears in viewer pane
7. **Performance**: 6 terminals open simultaneously -> no frame drops on VNC display
