# Sprite Management UI — Full API Coverage

**Date**: 2026-02-19
**Status**: APPROVED
**Scope**: Full Sprites.dev API coverage — CRUD, checkpoints, exec sessions, services

## Problem

The Sprites tab in Swarm-UI lists sprites and can open terminals, but has zero management controls. All 16 Tauri backend commands exist and work, but the frontend exposes none of them. Users cannot create, delete, checkpoint, or manage services from the UI.

## API Surface

| Group | Endpoint | Method | Tauri Cmd Exists? | Store Exists? |
|---|---|---|---|---|
| Sprites | POST /v1/sprites | REST | Yes | Yes |
| Sprites | GET /v1/sprites | REST | Yes | Yes |
| Sprites | GET /v1/sprites/{name} | REST | No | No |
| Sprites | PUT /v1/sprites/{name} | REST | No | No |
| Sprites | DELETE /v1/sprites/{name} | REST | Yes | Yes |
| Checkpoints | POST /v1/sprites/{name}/checkpoint | NDJSON | Yes (broken) | Yes (broken) |
| Checkpoints | GET /v1/sprites/{name}/checkpoints | REST | Yes | No |
| Checkpoints | POST /v1/sprites/{name}/checkpoints/{id}/restore | NDJSON | Yes (broken) | No |
| Exec | WSS /v1/sprites/{name}/exec | WS | Yes | Yes |
| Exec | GET /v1/sprites/{name}/exec | REST | No | No |
| Exec | POST /v1/sprites/{name}/exec/{id}/kill | NDJSON | No | No |
| Services | GET /v1/sprites/{name}/services | REST | No | No |
| Services | PUT /v1/sprites/{name}/services/{id} | REST | No | No |
| Services | POST /v1/sprites/{name}/services/{id}/start | NDJSON | No | No |
| Services | POST /v1/sprites/{name}/services/{id}/stop | NDJSON | No | No |
| Services | GET /v1/sprites/{name}/services/{id}/logs | NDJSON | No | No |

## Architecture

### Layer 1: Rust Backend

**New crate**: `reqwest-streams = { version = "0.8", features = ["json"] }` for NDJSON parsing.

**Fix existing commands** (checkpoint create/restore): Replace `resp.json()` with NDJSON stream via `Channel<StreamEvent>`.

**New commands** (10):
- `sprite_get`, `sprite_update` — REST, simple return
- `sprite_list_exec_sessions` — REST
- `sprite_kill_exec_session` — NDJSON via Channel
- `sprite_list_services` — REST
- `sprite_start_service`, `sprite_stop_service`, `sprite_get_service_logs` — NDJSON via Channel

### Layer 2: Types + Store

**Per-entity operation state** (not global):
```
operationState: Record<string, { loading, progress[], error }>
checkpoints: Record<string, Checkpoint[]>
execSessions: Record<string, ExecSession[]>
services: Record<string, Service[]>
serviceLogs: Record<string, ServiceStreamEvent[]>  // capped at 1000
```

**Modal store**: Separate `useModalStore` with `confirm()` and `prompt()`.

### Layer 3: Components

**SpriteCard collapsed**: Status badge (cold=gray, warm=amber, running=green) + action buttons (Terminal, Checkpoint, Delete)

**SpriteCard expanded** (4 tabs):
- Info: id, url, url_settings toggle, timestamps
- Checkpoints: create + list with restore, NDJSON progress inline
- Sessions: real exec sessions, kill button
- Services: start/stop, inline log viewer

### Key Patterns

- Per-entity loading state via `operationState[spriteName:operation]`
- Optimistic delete with rollback
- On-demand data fetch when tab opens
- NDJSON progress inline in card (not modal)
- Service logs: ring buffer capped at 1000 lines
- Tauri v2 Channel: `import { Channel } from '@tauri-apps/api/core'`

## Implementation Order

1. TypeScript types
2. Fix Rust NDJSON handlers (checkpoint create/restore)
3. Add missing Rust commands (6 REST + 4 NDJSON)
4. Extend Zustand store
5. Add modal store
6. Build SpriteActionBar
7. Build SpriteExpandedPanel (4-tab)
8. Wire CheckpointList
9. Wire ExecSessionList
10. Wire ServiceList + log viewer
