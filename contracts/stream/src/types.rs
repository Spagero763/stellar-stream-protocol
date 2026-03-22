#![no_std]
use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum StreamStatus {
    Active,
    Cancelled,
    Completed,
    PendingCancellation,  // in arbitration window
}

#[contracttype]
pub struct Stream {
    pub id: u64,
    pub sender: Address,
    pub recipient: Address,
    pub token: Address,
    pub deposit: u128,
    /// Tokens per second, scaled to token's decimal precision.
    /// Derived as: (deposit - cliff_amount) / (stop_time - effective_start)
    /// A value of 0 means cliff-only — no linear component.
    pub rate_per_second: u128,
    /// Lump-sum unlocked at cliff_time. Can be 0.
    pub cliff_amount: u128,
    /// Unix timestamp (seconds). Must be >= contract deployment time.
    pub start_time: u64,
    /// Unix timestamp. cliff_time == start_time means no cliff.
    pub cliff_time: u64,
    /// Unix timestamp. Must be > cliff_time.
    pub stop_time: u64,
    /// Cumulative amount already withdrawn by recipient.
    pub withdrawn_amount: u128,
    pub status: StreamStatus,
    /// If false, sender cannot call cancel(). Set permanently by renounce().
    pub cancellable: bool,
}