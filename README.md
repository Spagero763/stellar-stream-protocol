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

## 🌊 Drips Wave Program

Register at [drips.network](https://www.drips.network): connect wallet → Claim GitHub.
Rewards stream automatically when PRs merge.

| Label | Reward |
|---|---|
| `drips:trivial` | $15 – $40 |
| `drips:medium` | $80 – $250 |
| `drips:high` | $300 – $700 |

**SLA:** 7 / 14 / 21 days. Comment to claim. One claim per contributor.

---

## Issues

### `drips:trivial`

---

**#1 — `[BUG]` `rate_per_second` truncates to zero for small deposits — error message gives no guidance**

When `create()` rejects with `StreamError::RateTruncatedToZero`, the client
receives a raw error code with no explanation of why or what the minimum
deposit is for a given duration.

Two fixes needed:
1. Add a `min_deposit_for_duration(duration_seconds: u64) -> u128` view function
   that returns the minimum deposit required to produce `rate_per_second >= 1`
2. Update the error rustdoc to reference this function and give an example

Also add a unit test that calls `create()` with exactly the minimum deposit
(should succeed) and one stroop below (should fail with `RateTruncatedToZero`).

---

**#2 — `[BUG]` `withdrawable_amount` returns wrong value when `cliff_time == stop_time`**

**Reproduction:**
```
create(deposit=1000, cliff_amount=1000, start=T, cliff=T+100, stop=T+100)
```
This is a cliff-only stream with no linear component. At `t = T+100`,
`elapsed = 0`, `streamed = cliff_amount + 0 = 1000`. That's correct.

But at `t = T+99` (one second before cliff), the formula returns `0`. Also
correct. The bug is at `t = T+101` — the formula enters the `else` branch
(`t >= stop_time` is false because `t < stop_time+1`), computes
`elapsed = 1`, returns `1000 + 1*rate` where `rate = 0`... actually this
is fine. Write the test anyway — this edge case is completely untested and
the analysis above may be wrong.

Add unit tests for every combination of cliff-only, linear-only, and cliff+linear
at the exact boundary timestamps. Show your work in the test comments.

---

**#3 — `[DOCS]` `renounce()` behavior after cancellation is not specified**

What happens if a sender calls `renounce()` on an already-cancelled stream?
What happens if they call `cancel()` after `renounce()`? The code should
handle both cleanly, but right now neither is documented nor tested.

Add explicit guards:
- `renounce()` on a cancelled stream → `StreamError::AlreadyCancelled`
- `cancel()` on a renounced stream → `StreamError::NotCancellable`

Update rustdoc on both functions to describe these transitions.
Add unit tests for each.

---

**#4 — `[CI]` `cargo clippy` currently shows 14 warnings — add as required CI gate**

Running `cargo clippy -- -D warnings` locally shows 14 warnings across
stream and lockup contract crates (mostly unused variables, redundant clones,
and missing `#[must_use]` on functions that return `Result`).

Add a `clippy` job to `.github/workflows/` that runs
`cargo clippy --all-targets -- -D warnings` and fails the build on any warning.
Fix all 14 existing warnings as part of this PR.

---

**#5 — `[CHORE]` `.env.example` is missing `SOROBAN_RPC_URL` and stream contract addresses**

The current `.env.example` only has `DATABASE_URL`. The indexer and deployment
scripts require at minimum:
```
SOROBAN_RPC_URL=
HORIZON_URL=
NETWORK_PASSPHRASE=
STREAM_CONTRACT_ID=
FEE_CONTROLLER_CONTRACT_ID=
STREAM_NFT_CONTRACT_ID=
ADMIN_SECRET_KEY=
```

Update `.env.example` with all vars, a comment explaining each, and valid
testnet defaults where applicable (public RPC URLs are fine to commit).

---

**#6 — `[DX]` Add `Makefile` with standardized dev commands**

New contributors have to read three different READMEs to figure out how to
build, test, and deploy. Add a root `Makefile` with at minimum:
```makefile
build          # cargo build --target wasm32-unknown-unknown --release
test           # cargo test --all
clippy         # cargo clippy -- -D warnings
deploy-testnet # node scripts/deploy/deploy_all.js --network testnet
clean          # cargo clean + rm -rf node_modules dist .next
```

Bonus: add a `make check` target that runs build + clippy + test in sequence
and is used as the default CI entry point.

---

### `drips:medium`

---

**#7 — `[FEATURE]` `batch_create` — up to 20 streams in a single transaction**

Payroll use case: a company wants to create streams for all employees in one
transaction rather than submitting 50 separate ones and paying fees 50 times.
```rust
pub fn batch_create(
    env: Env,
    streams: Vec<BatchStreamParams>,
) -> Result<Vec<u64>, StreamError>
```

where `BatchStreamParams` contains the same fields as `create()`.

Requirements:
- Hard cap of 20 per call (`StreamError::BatchLimitExceeded` above that)
- The whole batch is atomic: if stream 7 of 15 fails validation, no streams
  are created and the full deposit is returned
- The total deposit across all streams is transferred in a single token call,
  not N separate calls
- Returns the Vec of created stream IDs in the same order as input
- Unit test: batch of 3 with one invalid (zero deposit) → all rejected
- Unit test: batch of 20 all valid → all created, IDs returned in order

---

**#8 — `[BUG]` `VestingChart` animation breaks when parent component re-renders**

**Reproduced in:** `CreateStreamForm` step 3 (review screen with embedded
`VestingChart`). When the user changes the recipient address field in step 1
and navigates back to step 3, the chart's `setInterval` is running multiple
concurrent instances because the component unmounts and remounts without
cleanup.

Symptoms: the "time cursor" line jumps erratically, console shows
`Warning: Can't perform a React state update on an unmounted component`.

Fix: return a cleanup function from the `useEffect` that runs the interval,
calling `clearInterval`. Add a test using `@testing-library/react` that
mounts → unmounts → remounts `VestingChart` and asserts no console errors.

---

**#9 — `[FEATURE]` Stream ownership transfer via `stream_nft`**

`transfer_stream_ownership(stream_id: u64, new_recipient: Address)`:

1. Verify `env.invoker() == stream.recipient`
2. Cross-contract call to `stream_contract.update_recipient(stream_id, new_recipient)`
3. Update `Map<u64, StreamRef>.recipient` in `stream_nft`
4. Emit `OwnershipTransferred { stream_id, old_recipient, new_recipient }`

The `stream` contract needs a new `update_recipient` entry point that only
accepts calls from the `stream_nft` contract address (stored in stream contract's
init). This is a privileged cross-contract call — the auth pattern matters.

Test: original recipient withdraws partial amount, transfers ownership,
new recipient withdraws remainder, original recipient's withdraw attempt fails.

---

**#10 — `[FEATURE]` `WithdrawPanel` component — real-time withdrawable balance, no estimates**

`WithdrawPanel` takes `streamId: string` and:

1. On mount, calls `simulateTransaction` for `withdrawable_amount(stream_id)` —
   this is the actual on-chain value, not a client-side estimate
2. Polls every 15 seconds with `setInterval` + `AbortController` cleanup
3. Shows: withdrawable now, total withdrawn, total deposit, % vested progress bar
4. Shows "Fully vested in X days Y hours" countdown when stream is active
5. `Withdraw [amount]` button: opens an amount input (default: max withdrawable),
   signs and submits with Freighter, shows tx status inline (not a modal)
6. Handles the case where `withdrawable_amount = 0` gracefully (button disabled,
   tooltip explains when next tokens unlock)

No external polling libraries. No estimates. All values from `simulateTransaction`.

---

**#11 — `[FEATURE]` `GET /streams/recipient/:address` with cursor pagination**

The indexer needs an endpoint the frontend actually calls.
```
GET /streams/recipient/:address?cursor=<stream_id>&limit=20&status=active
```

Response:
```json
{
  "streams": [
    {
      "id": 42,
      "sender": "G...",
      "token": "C...",
      "token_symbol": "USDC",
      "deposit": "50000000000",
      "withdrawn_amount": "12500000000",
      "withdrawable_now": "3125000000",
      "rate_per_second": "1984126",
      "start_time": 1700000000,
      "cliff_time": 1702678400,
      "stop_time": 1731600000,
      "status": "active"
    }
  ],
  "next_cursor": 41,
  "has_more": true
}
```

`withdrawable_now` is computed server-side using the same formula as the
contract — not queried from chain. `cursor` is the last `id` received.
Add `GET /streams/sender/:address` with the same interface.
Integration tests with seeded fixture data covering all status values.

---

**#12 — `[PERF]` Indexer replays all historical events on every restart**

The indexer currently starts from ledger 0 on every process restart.
On testnet with 3 months of history this takes 4+ minutes and hammers
the Horizon rate limit.

Implement checkpoint persistence:
- Store `last_processed_ledger: u32` in a `indexer_state` table after
  processing each batch of events
- On startup, read this value and start streaming from `last_processed_ledger + 1`
- Handle the edge case where a checkpoint exists but the ledger is no longer
  available on Horizon (purged from history) — fall back to earliest available

Add integration test: start indexer, process 100 mock events, kill and restart,
assert it resumes from ledger 101 and doesn't reprocess the first 100.

---

### `drips:high`

---

**#13 — `[FEATURE]` Cancellation dispute window with arbitration state machine**

For employment and grant contexts, a unilateral cancel by the sender is a
contentious operation — the recipient may dispute whether the cancellation
was valid (e.g. termination without cause should still pay out cliff).

Add `arbitration_window_seconds: u64` as an optional stream parameter
(0 = no dispute window, instant cancel).

State transitions:
```
Active ──cancel()──▶ PendingCancellation (locked for arbitration_window)
                          │
              dispute()   │  window expires (no dispute)
                 ▼        ▼
           Disputed    Cancelled
               │
         resolve(recipient_amount)  ← arbitrator only
               ▼
           Settled (recipient gets recipient_amount, sender gets remainder)
```

`arbitrator: Address` is set at stream creation. Can be a multisig,
a DAO contract, or `Address::zero()` (which means no arbitration possible
even if window > 0 — document this footgun).

All four state transitions need events. All six error cases need tested.
This is a full state machine implementation — not a patch on top of cancel().

---

**#14 — `[SECURITY]` `renounce()` does not check if stream is already completed**

**Severity: Medium**

If a stream's `stop_time` has passed and the recipient has already withdrawn
the full `deposit`, calling `renounce()` still succeeds and emits
`StreamRenounced`. This is harmless but incorrect — you cannot renounce
cancellation rights on a completed stream because there's nothing left to cancel.

More importantly: what if `renounce()` is called on a stream where
`withdrawn_amount < deposit` but `current_time > stop_time`? The recipient
can still call `withdraw()` to get the remainder, but the stream is
functionally complete. Should renounce be allowed here?

This issue requires a design decision before implementation:

**Option A:** Block `renounce()` if `status != Active`. Clean, simple.

**Option B:** Allow `renounce()` on active and completed streams (since it's
idempotent from a security perspective), but emit a `[WARN]` event.

Open a design discussion in the issue comments, reach a decision, implement it,
and update the rustdoc with the rationale.

---

**#15 — `[FEATURE]` End-to-end payroll integration test — 10 employees, 6-month vest, early termination**

In `tests/integration/payroll_simulation.rs`, write a deterministic simulation
that exercises the entire stream protocol under realistic conditions:

Setup:
- Deploy all 4 contracts on local Soroban sandbox
- Create a test token with 7 decimal places (XLM-like)
- Fund a "company" account with 600,000 tokens

Scenario:
1. `batch_create` 10 streams: 1-month cliff (10% of total), 6-month linear vest
2. Advance ledger time to T+2 weeks — no one can withdraw (pre-cliff)
3. Assert `withdrawable_amount` returns 0 for all 10
4. Advance to T+1 month — cliff unlocks for all 10
5. 5 employees withdraw exactly their cliff amount
6. Advance to T+3 months
7. Employee #6 is "terminated" (stream cancelled by sender)
8. Verify employee #6 receives pro-rated amount up to cancellation time
9. Verify sender receives correct remainder
10. Advance to T+6 months — stream ends
11. Remaining 9 employees withdraw full balance
12. Assert company account balance == 0 (all tokens disbursed correctly)
13. Assert no stream has `deposit - withdrawn_amount > 0` (no stuck funds)

Every assertion should include the expected value and the actual value in
the failure message. This is a regression suite — it must stay green through
any refactor of the streaming math.

---

## Project Structure
```
stellar-stream-protocol/
├── contracts/
│   ├── stream/src/
│   │   ├── lib.rs          # create, withdraw, cancel, renounce, batch_create
│   │   ├── linear.rs       # linear component of withdrawable_amount
│   │   ├── cliff.rs        # cliff lump-sum logic
│   │   ├── state.rs        # StreamStatus enum, state transition guards
│   │   ├── errors.rs
│   │   ├── events.rs
│   │   └── types.rs
│   ├── lockup/src/
│   │   └── lib.rs
│   ├── fee_controller/src/
│   │   └── lib.rs
│   └── stream_nft/src/
│       └── lib.rs
├── frontend/src/components/
│   ├── StreamCard/
│   ├── CreateStreamForm/
│   ├── VestingChart/
│   ├── StreamTimeline/
│   └── WithdrawPanel/
├── indexer/src/
│   ├── handlers/
│   ├── db/
│   └── api/
└── tests/
    ├── unit/
    ├── fuzz/
    └── integration/
```

## License
Apache-2.0