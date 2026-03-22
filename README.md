# stellar-stream-protocol

Real-time token streaming on Soroban.
Cliff + linear vesting. Cancellable, renounce-able, transferable positions.
Built for payroll, DAO contributor grants, and investor vesting schedules.

---

## How Streams Work

A stream is a struct stored on-chain. It has a `deposit`, a `rate_per_second`,
an optional `cliff_amount`, and a time window. At any moment `t`, the recipient
can withdraw exactly:
```
if t < cliff_time:
    withdrawable = 0

elif t >= stop_time:
    withdrawable = deposit - withdrawn_amount

else:
    elapsed = t - max(start_time, cliff_time)
    streamed = cliff_amount + (elapsed * rate_per_second)
    withdrawable = min(streamed, deposit) - withdrawn_amount
```

The contract holds the deposit in escrow. There is no off-chain component —
the stream balance is deterministically computable from on-chain state alone.

---

## Known Limitation: Integer Truncation

`rate_per_second` is a `u128` integer. For very small deposits over very long
durations, integer division can truncate `rate_per_second` to zero. For example:
```
deposit = 1 XLM (10_000_000 stroops)
duration = 10 years (315_360_000 seconds)
rate = 10_000_000 / 315_360_000 = 0 (truncated)
```

The contract rejects these with `StreamError::RateTruncatedToZero`. The minimum
viable stream is documented in the `create()` rustdoc. For sub-stroop precision,
use a token with more decimals.

---

## Stream Ownership and OTC Transfers

`stream_nft` tracks recipient ownership. `transfer_stream_ownership(stream_id,
new_recipient)` changes who can call `withdraw` and `cancel`. This enables
OTC secondary markets for vesting positions — a common need for early
contributors who need liquidity before their vest completes.

---

