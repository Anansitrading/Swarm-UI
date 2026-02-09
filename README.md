# Swarm-UI

Cross-platform desktop app for monitoring and steering Claude Code agent sessions, teams, and [Sprites](https://sprites.dev) VMs.

Built with [Tauri 2](https://tauri.app) (Rust + React + TypeScript).

## Features

- **Session Monitor** - Real-time view of all Claude Code sessions with status, context usage, model info, and conversation history
- **Agent Picker** - Launch new Claude Code sessions with any agent (`claude`, `oracle`, `smith`, `trinity`, or custom agents from `~/.claude/agents/`)
- **Steering Input** - Inject context, files, and commands into running sessions
- **Teams View** - Monitor agent teams with task progress, member status, and session sync
- **Sprites Integration** - Manage remote Sprites VMs: terminal access, session monitoring, checkpoint management
- **Git Diff Viewer** - Inline diff viewer for session working directories
- **Smith Overrides** - Per-session Smith configuration for custom guardrails

## Install

### Pre-built Binaries

Download from [Releases](https://github.com/Anansitrading/Swarm-UI/releases):

| Platform | File | Notes |
|----------|------|-------|
| **Windows** | `Swarm-UI_x.x.x_x64-setup.exe` | NSIS installer, no admin required |
| **macOS** | `Swarm-UI_x.x.x_x64.dmg` | Drag to Applications |
| **Linux (Debian/Ubuntu)** | `Swarm-UI_x.x.x_amd64.deb` | `sudo dpkg -i Swarm-UI_*.deb` |
| **Linux (Fedora/RHEL)** | `Swarm-UI_x.x.x.x86_64.rpm` | `sudo rpm -i Swarm-UI_*.rpm` |
| **Linux (AppImage)** | `Swarm-UI_x.x.x_amd64.AppImage` | `chmod +x` and run |

### Build from Source

#### Prerequisites

- [Node.js](https://nodejs.org) >= 18
- [Rust](https://rustup.rs) >= 1.77
- Platform-specific dependencies (see below)

#### Windows

1. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with "Desktop development with C++" workload
2. Install [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (included in Windows 10 21H2+ and Windows 11)

```powershell
git clone https://github.com/Anansitrading/Swarm-UI.git
cd Swarm-UI
npm install
npx tauri build
```

The installer will be at `src-tauri/target/release/bundle/nsis/Swarm-UI_*_x64-setup.exe`.

#### macOS

```bash
xcode-select --install  # if not already installed
git clone https://github.com/Anansitrading/Swarm-UI.git
cd Swarm-UI
npm install
npx tauri build
```

#### Linux (Debian/Ubuntu)

```bash
sudo apt install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
git clone https://github.com/Anansitrading/Swarm-UI.git
cd Swarm-UI
npm install
npx tauri build
```

### Development

```bash
npm install
npx tauri dev
```

The app opens at `http://localhost:1420` with hot-reload. The Rust backend recompiles on changes.

## Configuration

### Claude Code

Swarm-UI monitors Claude Code sessions by watching `~/.claude/projects/` for JSONL session files. No configuration needed - just run Claude Code sessions and they appear automatically.

On Windows, this is `%USERPROFILE%\.claude\projects\`.

### Custom Agents

Place agent definition files in `~/.claude/agents/` (or `%USERPROFILE%\.claude\agents\` on Windows). Each `.md` file becomes an option in the Agent Picker.

### Sprites

Configure Sprites API access in Settings tab:
- **API URL**: Your Sprites deployment URL
- **API Token**: Authentication token

### Smith Overrides

Per-session Smith overrides are stored in `~/.claude/smith-overrides/`. Use the gear icon in session detail to configure.

## Architecture

```
src/                  # React frontend (TypeScript)
  components/
    layout/           # AppShell, Sidebar, Toolbar, PaneGrid
    session/          # SessionDetail, SessionCard, SteeringInput, SmithPanel
    team/             # TeamList
    sprite/           # SpriteList, SpriteGrid
    terminal/         # TerminalPane, AgentPicker
    diff/             # DiffViewer
  stores/             # Zustand state (session, team, sprite, layout, terminal)

src-tauri/            # Rust backend
  src/
    commands/          # Tauri IPC commands (session, pty, process, team, agent, sprite)
    parsers/           # JSONL session parser
    watchers/          # Filesystem watchers (sessions, teams)
```

## License

Proprietary - Anansitrading
