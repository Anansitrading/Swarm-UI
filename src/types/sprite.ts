export interface SpriteInfo {
    name: string;
    status: string;
    id?: string;
    region?: string;
}

export interface BotSlot {
    slot: number;
    bot_name: string | null;
    sprite_name: string | null;
    status: string;
    ticket_id: string | null;
    role: string | null;
    claimed_at: string | null;
    heartbeat: string | null;
}

export interface PoolState {
    slots: BotSlot[];
    total: number;
    active: number;
    idle: number;
}
