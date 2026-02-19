import { create } from "zustand";
import { invoke, Channel } from "@tauri-apps/api/core";
import type {
    SpriteInfo,
    SpriteDetail,
    Checkpoint,
    ExecSession,
    Service,
    StreamEvent,
    ServiceStreamEvent,
    ExecKillEvent,
    OperationState,
} from "../types/sprite";

const DEFAULT_OP: OperationState = { loading: false, progress: [], error: null };
const MAX_LOG_LINES = 1000;

function opKey(sprite: string, operation: string): string {
    return `${sprite}:${operation}`;
}

interface SpriteState {
    // Core data
    sprites: SpriteInfo[];
    loading: boolean;
    error: string | null;

    // Per-entity caches
    details: Record<string, SpriteDetail>;
    checkpoints: Record<string, Checkpoint[]>;
    execSessions: Record<string, ExecSession[]>;
    services: Record<string, Service[]>;
    serviceLogs: Record<string, ServiceStreamEvent[]>;

    // Per-entity operation state keyed by "spriteName:operation"
    ops: Record<string, OperationState>;

    // ── Core CRUD ─────────────────────────────────────────────
    fetchSprites: () => Promise<void>;
    getSprite: (name: string) => Promise<SpriteDetail>;
    createSprite: (name: string) => Promise<void>;
    updateSprite: (name: string, urlAuth: string) => Promise<void>;
    deleteSprite: (name: string) => Promise<void>;

    // ── Exec ──────────────────────────────────────────────────
    execOnSprite: (name: string, command: string) => Promise<string>;
    listExecSessions: (name: string) => Promise<void>;
    killExecSession: (
        name: string,
        sessionId: string,
        signal?: string,
    ) => Promise<void>;

    // ── Checkpoints ───────────────────────────────────────────
    listCheckpoints: (name: string) => Promise<void>;
    createCheckpoint: (name: string, comment?: string) => Promise<void>;
    restoreCheckpoint: (
        name: string,
        checkpointId: string,
    ) => Promise<void>;

    // ── Services ──────────────────────────────────────────────
    listServices: (name: string) => Promise<void>;
    startService: (name: string, serviceName: string) => Promise<void>;
    stopService: (name: string, serviceName: string) => Promise<void>;
    getServiceLogs: (
        name: string,
        serviceName: string,
        lines?: number,
    ) => Promise<void>;

    // ── Helpers ───────────────────────────────────────────────
    clearError: () => void;
    getOp: (key: string) => OperationState;
}

export const useSpriteStore = create<SpriteState>((set, get) => ({
    sprites: [],
    loading: false,
    error: null,
    details: {},
    checkpoints: {},
    execSessions: {},
    services: {},
    serviceLogs: {},
    ops: {},

    getOp: (key: string) => get().ops[key] ?? DEFAULT_OP,

    // ── Core CRUD ─────────────────────────────────────────────────────

    fetchSprites: async () => {
        set({ loading: true, error: null });
        try {
            const sprites = await invoke<SpriteInfo[]>("sprite_list");
            set({ sprites, loading: false });
        } catch (e) {
            set({ loading: false, error: String(e) });
        }
    },

    getSprite: async (name: string) => {
        const key = opKey(name, "detail");
        set((s) => ({
            ops: { ...s.ops, [key]: { loading: true, progress: [], error: null } },
        }));
        try {
            const detail = await invoke<SpriteDetail>("sprite_get", { name });
            set((s) => ({
                details: { ...s.details, [name]: detail },
                ops: { ...s.ops, [key]: { ...DEFAULT_OP } },
            }));
            return detail;
        } catch (e) {
            set((s) => ({
                ops: {
                    ...s.ops,
                    [key]: { loading: false, progress: [], error: String(e) },
                },
            }));
            throw e;
        }
    },

    createSprite: async (name: string) => {
        const sprite = await invoke<SpriteInfo>("sprite_create", { name });
        set((s) => ({ sprites: [...s.sprites, sprite] }));
    },

    updateSprite: async (name: string, urlAuth: string) => {
        const detail = await invoke<SpriteDetail>("sprite_update", {
            name,
            urlAuth,
        });
        set((s) => ({ details: { ...s.details, [name]: detail } }));
    },

    deleteSprite: async (name: string) => {
        const prev = get().sprites;
        // Optimistic delete
        set((s) => ({ sprites: s.sprites.filter((sp) => sp.name !== name) }));
        try {
            await invoke("sprite_delete", { name });
        } catch (e) {
            // Rollback on failure
            set({ sprites: prev });
            throw e;
        }
    },

    // ── Exec ──────────────────────────────────────────────────────────

    execOnSprite: async (name: string, command: string) => {
        return await invoke<string>("sprite_exec", { name, command });
    },

    listExecSessions: async (name: string) => {
        const key = opKey(name, "exec-sessions");
        set((s) => ({
            ops: { ...s.ops, [key]: { loading: true, progress: [], error: null } },
        }));
        try {
            const sessions = await invoke<ExecSession[]>(
                "sprite_list_exec_sessions",
                { name },
            );
            set((s) => ({
                execSessions: { ...s.execSessions, [name]: sessions },
                ops: { ...s.ops, [key]: { ...DEFAULT_OP } },
            }));
        } catch (e) {
            set((s) => ({
                ops: {
                    ...s.ops,
                    [key]: { loading: false, progress: [], error: String(e) },
                },
            }));
        }
    },

    killExecSession: async (
        name: string,
        sessionId: string,
        signal?: string,
    ) => {
        const key = opKey(name, `kill-${sessionId}`);
        set((s) => ({
            ops: { ...s.ops, [key]: { loading: true, progress: [], error: null } },
        }));

        const onEvent = new Channel<ExecKillEvent>();
        onEvent.onmessage = (event) => {
            const msg = event.message ?? event.signal ?? event.type;
            set((s) => ({
                ops: {
                    ...s.ops,
                    [key]: {
                        ...s.ops[key],
                        progress: [...(s.ops[key]?.progress ?? []), msg],
                    },
                },
            }));
        };

        try {
            await invoke("sprite_kill_exec_session", {
                name,
                sessionId,
                signal,
                onEvent,
            });
            set((s) => ({ ops: { ...s.ops, [key]: { ...DEFAULT_OP } } }));
            // Refresh exec sessions after kill
            get().listExecSessions(name);
        } catch (e) {
            set((s) => ({
                ops: {
                    ...s.ops,
                    [key]: {
                        loading: false,
                        progress: s.ops[key]?.progress ?? [],
                        error: String(e),
                    },
                },
            }));
        }
    },

    // ── Checkpoints ───────────────────────────────────────────────────

    listCheckpoints: async (name: string) => {
        const key = opKey(name, "checkpoints");
        set((s) => ({
            ops: { ...s.ops, [key]: { loading: true, progress: [], error: null } },
        }));
        try {
            const cps = await invoke<Checkpoint[]>("sprite_list_checkpoints", {
                name,
            });
            set((s) => ({
                checkpoints: { ...s.checkpoints, [name]: cps },
                ops: { ...s.ops, [key]: { ...DEFAULT_OP } },
            }));
        } catch (e) {
            set((s) => ({
                ops: {
                    ...s.ops,
                    [key]: { loading: false, progress: [], error: String(e) },
                },
            }));
        }
    },

    createCheckpoint: async (name: string, comment?: string) => {
        const key = opKey(name, "checkpoint-create");
        set((s) => ({
            ops: { ...s.ops, [key]: { loading: true, progress: [], error: null } },
        }));

        const onEvent = new Channel<StreamEvent>();
        onEvent.onmessage = (event) => {
            const msg = event.data ?? event.error ?? event.type;
            set((s) => ({
                ops: {
                    ...s.ops,
                    [key]: {
                        ...s.ops[key],
                        progress: [...(s.ops[key]?.progress ?? []), msg],
                    },
                },
            }));
        };

        try {
            await invoke("sprite_checkpoint_create", { name, comment, onEvent });
            set((s) => ({ ops: { ...s.ops, [key]: { ...DEFAULT_OP } } }));
            // Refresh checkpoints list after create
            get().listCheckpoints(name);
        } catch (e) {
            set((s) => ({
                ops: {
                    ...s.ops,
                    [key]: {
                        loading: false,
                        progress: s.ops[key]?.progress ?? [],
                        error: String(e),
                    },
                },
            }));
        }
    },

    restoreCheckpoint: async (name: string, checkpointId: string) => {
        const key = opKey(name, "checkpoint-restore");
        set((s) => ({
            ops: { ...s.ops, [key]: { loading: true, progress: [], error: null } },
        }));

        const onEvent = new Channel<StreamEvent>();
        onEvent.onmessage = (event) => {
            const msg = event.data ?? event.error ?? event.type;
            set((s) => ({
                ops: {
                    ...s.ops,
                    [key]: {
                        ...s.ops[key],
                        progress: [...(s.ops[key]?.progress ?? []), msg],
                    },
                },
            }));
        };

        try {
            await invoke("sprite_restore_checkpoint", {
                name,
                checkpointId,
                onEvent,
            });
            set((s) => ({ ops: { ...s.ops, [key]: { ...DEFAULT_OP } } }));
            get().fetchSprites();
            get().listCheckpoints(name);
        } catch (e) {
            // Check if we got progress events despite the error
            // (connection closed after server sent complete — restore likely succeeded)
            const op = get().ops[key];
            const hasProgress = (op?.progress?.length ?? 0) > 0;
            if (hasProgress) {
                set((s) => ({ ops: { ...s.ops, [key]: { ...DEFAULT_OP } } }));
                get().fetchSprites();
            } else {
                set((s) => ({
                    ops: {
                        ...s.ops,
                        [key]: {
                            loading: false,
                            progress: s.ops[key]?.progress ?? [],
                            error: String(e),
                        },
                    },
                }));
            }
        }
    },

    // ── Services ──────────────────────────────────────────────────────

    listServices: async (name: string) => {
        const key = opKey(name, "services");
        set((s) => ({
            ops: { ...s.ops, [key]: { loading: true, progress: [], error: null } },
        }));
        try {
            const svcs = await invoke<Service[]>("sprite_list_services", {
                name,
            });
            set((s) => ({
                services: { ...s.services, [name]: svcs },
                ops: { ...s.ops, [key]: { ...DEFAULT_OP } },
            }));
        } catch (e) {
            set((s) => ({
                ops: {
                    ...s.ops,
                    [key]: { loading: false, progress: [], error: String(e) },
                },
            }));
        }
    },

    startService: async (name: string, serviceName: string) => {
        const key = opKey(name, `service-start-${serviceName}`);
        set((s) => ({
            ops: { ...s.ops, [key]: { loading: true, progress: [], error: null } },
        }));

        const onEvent = new Channel<ServiceStreamEvent>();
        onEvent.onmessage = (event) => {
            const msg = event.data ?? event.type;
            set((s) => ({
                ops: {
                    ...s.ops,
                    [key]: {
                        ...s.ops[key],
                        progress: [...(s.ops[key]?.progress ?? []), msg],
                    },
                },
            }));
        };

        try {
            await invoke("sprite_start_service", {
                name,
                serviceName,
                onEvent,
            });
            set((s) => ({ ops: { ...s.ops, [key]: { ...DEFAULT_OP } } }));
            get().listServices(name);
        } catch (e) {
            set((s) => ({
                ops: {
                    ...s.ops,
                    [key]: {
                        loading: false,
                        progress: s.ops[key]?.progress ?? [],
                        error: String(e),
                    },
                },
            }));
        }
    },

    stopService: async (name: string, serviceName: string) => {
        const key = opKey(name, `service-stop-${serviceName}`);
        set((s) => ({
            ops: { ...s.ops, [key]: { loading: true, progress: [], error: null } },
        }));

        const onEvent = new Channel<ServiceStreamEvent>();
        onEvent.onmessage = (event) => {
            const msg = event.data ?? event.type;
            set((s) => ({
                ops: {
                    ...s.ops,
                    [key]: {
                        ...s.ops[key],
                        progress: [...(s.ops[key]?.progress ?? []), msg],
                    },
                },
            }));
        };

        try {
            await invoke("sprite_stop_service", {
                name,
                serviceName,
                onEvent,
            });
            set((s) => ({ ops: { ...s.ops, [key]: { ...DEFAULT_OP } } }));
            get().listServices(name);
        } catch (e) {
            set((s) => ({
                ops: {
                    ...s.ops,
                    [key]: {
                        loading: false,
                        progress: s.ops[key]?.progress ?? [],
                        error: String(e),
                    },
                },
            }));
        }
    },

    getServiceLogs: async (
        name: string,
        serviceName: string,
        lines?: number,
    ) => {
        const logKey = `${name}:${serviceName}`;
        const key = opKey(name, `service-logs-${serviceName}`);
        set((s) => ({
            serviceLogs: { ...s.serviceLogs, [logKey]: [] },
            ops: { ...s.ops, [key]: { loading: true, progress: [], error: null } },
        }));

        const onEvent = new Channel<ServiceStreamEvent>();
        onEvent.onmessage = (event) => {
            set((s) => {
                const existing = s.serviceLogs[logKey] ?? [];
                // Ring buffer capped at MAX_LOG_LINES
                const updated = [...existing, event].slice(-MAX_LOG_LINES);
                return { serviceLogs: { ...s.serviceLogs, [logKey]: updated } };
            });
        };

        try {
            await invoke("sprite_get_service_logs", {
                name,
                serviceName,
                lines,
                onEvent,
            });
            set((s) => ({ ops: { ...s.ops, [key]: { ...DEFAULT_OP } } }));
        } catch (e) {
            set((s) => ({
                ops: {
                    ...s.ops,
                    [key]: { loading: false, progress: [], error: String(e) },
                },
            }));
        }
    },

    // ── Helpers ────────────────────────────────────────────────────────

    clearError: () => set({ error: null }),
}));
