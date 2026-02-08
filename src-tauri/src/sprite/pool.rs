/// Pool state parsing is handled in watchers/pool_watcher.rs
/// This module is reserved for pool-specific business logic
/// (claim/release/rotate) if needed in the future.

pub use crate::watchers::pool_watcher::{BotSlot, PoolState};
