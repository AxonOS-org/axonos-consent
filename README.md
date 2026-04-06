# axonos-consent

[![CI](https://github.com/AxonOS-org/axonos-consent/actions/workflows/ci.yml/badge.svg)](https://github.com/AxonOS-org/axonos-consent/actions)

**MMP Consent Extension v0.1.0 — reference implementation.**

Zero-alloc. Bounded. Fuzz-tested. `#![forbid(unsafe_code)]`.

Spec: [sym.bot/spec/mmp-consent](https://sym.bot/spec/mmp-consent) · Version: `1`

---

## API

```rust
// Single entry point: wire bytes → validated state transition
let result = engine.process_raw(&peer_id, cbor_bytes, now_us)?;
// result.new_state: ConsentState
// result.warnings: SHOULD-level advisories
```

`process_raw()` is the **only** function external code calls. It executes:

```text
CBOR decode (bounded) → invariant check (MUST/SHOULD) → state transition (exhaustive) → StimGuard
```

No other function combination is needed or recommended.

---

## Guarantees

| Property | Guarantee | Evidence |
|----------|-----------|---------|
| No heap allocation | Critical path is `#![no_std]`, no `alloc` | `ReasonBuf` (64B fixed), encoder writes to `&mut [u8]` |
| Bounded parsing | All inputs bounded | `MAX_MAP_FIELDS=8`, `MAX_STRING_LEN=128`, `MAX_NESTING=4` |
| Deterministic execution | No loops beyond field count | O(n), n≤8 fields. No recursion in decode path |
| No unsafe code | `#![forbid(unsafe_code)]` | Compile-time enforced |
| Exhaustive state machine | All 9 cells explicit | `apply_frame()`: 3 states × 3 frames, zero wildcards |
| Silent error impossible | `#[must_use]` on `Error` and transitions | Compile warning if Result ignored |
| Forward compatible | Unknown CBOR keys skipped | `skip_value()` with same bounds |

---

## State machine (§4)

```text
 ┌─────────┐  consent-suspend   ┌───────────┐
 │ GRANTED │ ─────────────────> │ SUSPENDED │
 │         │ <───────────────── │           │
 └────┬────┘  consent-resume    └─────┬─────┘
      │                               │
      │  consent-withdraw             │  consent-withdraw
      v                               v
 ┌──────────────────────────────────────┐
 │          WITHDRAWN (terminal)         │
 └──────────────────────────────────────┘
```

### Transition table (exhaustive, no wildcards)

| Current \ Frame | Withdraw | Suspend | Resume |
|-----------------|----------|---------|--------|
| **GRANTED** | → WITHDRAWN | → SUSPENDED | → GRANTED *(idempotent)* |
| **SUSPENDED** | → WITHDRAWN | → SUSPENDED *(idempotent)* | → GRANTED |
| **WITHDRAWN** | **REJECT** | **REJECT** | **REJECT** |

**Formal closure:** `apply_frame()` matches on `(ConsentState, ConsentFrame)` exhaustively. Adding a new state or frame variant produces a compile error, not a runtime bug.

---

## Threat model

| Threat | Vector | Mitigation | Bound |
|--------|--------|------------|-------|
| Map bomb | CBOR map(10⁶) | `MAX_MAP_FIELDS` | 8 |
| String bomb | text(1MB) | `MAX_STRING_LEN` | 128 B |
| Stack overflow | Nested maps | `MAX_NESTING_DEPTH` | 4 |
| Type confusion | `{"type":"withdraw","type":"resume"}` | Bitmask dedup | 7 keys |
| Unsupported CBOR | Neg int, tag, float | Explicit reject | Types 1,2,4,6,7 |
| Buffer overflow | Large encode | `Result<_, BufferTooSmall>` | 256 B max |
| State violation | WITHDRAWN → RESUME | `apply_frame()` reject | Compile-checked |
| Silent error | Unchecked Result | `#[must_use]` | Compile warning |

---

## Complexity

```text
process_raw():
  decode:     O(n), n ≤ MAX_MAP_FIELDS = 8
              Per field: 1 key read + 1 value read = O(1)
              skip_value: bounded by MAX_NESTING × MAX_MAP_FIELDS
  invariants: O(1) — fixed field checks, no loops
  transition: O(1) — single exhaustive match

Total: O(n), n ≤ 8. Worst-case: 16 CBOR reads + 7 invariant checks + 1 match.
WCET target: <10µs on Cortex-M4F @ 168 MHz.

encode(): O(k), k = present fields ≤ 7. Zero allocation.
StimGuard path: <1µs (state write + DAC register write, non-preemptible).
```

---

## Error taxonomy

```text
L1 (Wire)    → Error::Decode     — malformed CBOR, bounds, unsupported types
L2 (Struct)  → Error::Invariant  — MUST violations (zero timestamp, reason too long)
L3 (State)   → Error::Transition — WITHDRAWN→any, peer not found
L4 (System)  → Error::Encode     — buffer too small
```

All error types: `Copy`, `#[must_use]`, `From` conversions to unified `Error`.

---

## Security bounds

| Constant | Value | Purpose |
|----------|-------|---------|
| `MAX_MAP_FIELDS` | 8 | Map bomb protection |
| `MAX_STRING_LEN` | 128 | String bomb protection |
| `MAX_NESTING_DEPTH` | 4 | Stack protection |
| `MAX_REASON_LEN` | 64 | ReasonBuf capacity |
| `MAX_ENCODED_SIZE` | 256 | Encoder ceiling |
| `MAX_PEERS` | 8 | Engine peer table |
| `CONSENT_PROTOCOL_VERSION` | 1 | Wire versioning |

---

## Spec-to-code mapping

| § | Module | Enforcement |
|---|--------|-------------|
| §3 | `frames` | Type-safe enum (Withdraw/Suspend/Resume) |
| §3.1 | `frames::ConsentWithdraw` | `scope`: non-optional |
| §3.1 | `invariants` | `timestamp`: SHOULD warning if absent |
| §3.4 | `reason` | `ReasonCode` enum, 0x00–0x0F spec / 0x10–0xFF impl |
| §4 | `state::apply_frame` | Exhaustive 3×3 table, WITHDRAWN terminal |
| §5.1 | `engine::process_raw` | Full pipeline: decode→validate→transition→StimGuard |
| §6.1 | `engine::allows_cognitive_frames` | `false` for SUSPENDED/WITHDRAWN |
| §6.4 | `state::to_gossip_bits` | 2-bit: 00/01/10 |
| §7 | `codec::cbor` | Bounded decoder, string-keyed map |
| §8 | `stim_guard` | DacGate trait, <1µs path, atomicity guaranteed |
| §10 | `invariants` | MUST → violation, SHOULD → warning |

---

## Duplicate key detection

Bitmask (documented in `decode()`):

```text
bit 0 (0x01) = "type"          §3
bit 1 (0x02) = "scope"         §3.1
bit 2 (0x04) = "reasonCode"    §3.4
bit 3 (0x08) = "reason"        §3.1
bit 4 (0x10) = "epoch"         §3.1
bit 5 (0x20) = "timestamp"     §3.1
bit 6 (0x40) = "timestamp_us"  AxonOS extension
```

---

## Testing

```bash
cargo test                  # no_std: CBOR, state machine, engine, invariants, process_raw
cargo test --features json  # + JSON round-trip against 15 interop vectors
cargo +nightly fuzz run fuzz_cbor_decode     # crash resistance
cargo +nightly fuzz run fuzz_cbor_roundtrip  # encode→decode invariant
```

60+ tests: 7 CBOR round-trips, 5 security rejections, 2 buffer overflow, 6 invariants, 9 exhaustive state table, 4 process_raw pipeline, 4 process_frame, 15 JSON vectors, 5 state machine, 5 engine, 3 error taxonomy.

---

## Licence

MIT

---

[axonos.org](https://axonos.org) · [medium.com/@AxonOS](https://medium.com/@AxonOS) · axonosorg@gmail.com
[sym.bot/spec/mmp](https://sym.bot/spec/mmp) · [github.com/sym-bot](https://github.com/sym-bot)
