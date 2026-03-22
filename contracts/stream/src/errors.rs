#![no_std]
use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum StreamError {
    StreamNotFound          = 1,
    NotSender               = 2,
    NotRecipient            = 3,
    AlreadyCancelled        = 4,
    NotCancellable          = 5,
    ZeroDeposit             = 6,
    InvalidTimeRange        = 7,
    /// cliff_time > stop_time
    CliffAfterStop          = 8,
    /// Attempted to withdraw more than withdrawable_amount
    InsufficientBalance     = 9,
    /// rate_per_second computed to 0 due to integer truncation
    /// (deposit too small for duration — see MIN_DEPOSIT docs)
    RateTruncatedToZero     = 10,
    BatchLimitExceeded      = 11,
    AlreadyRenounced        = 12,
}