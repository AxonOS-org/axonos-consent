# axonos-consent
[![Rust Version](https://img.shields.io/badge/rust-no__std-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Target](https://img.shields.io/badge/target-Cortex--M4F-blue?style=flat-square)](https://developer.arm.com/Processors/Cortex-M4)
[![Fuzzing](https://img.shields.io/badge/security-fuzzing--verified-brightgreen?style=flat-square)](https://github.com/google/fuzzing)
[![Safety](https://img.shields.io/badge/memory-zero--alloc-black?style=flat-square)](https://axonos.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-lightgrey?style=flat-square)](LICENSE)



**MMP Consent Extension v0.2.0 — reference implementation.**
Zero-alloc. Security-bounded. Fuzz-tested.

Spec: [sym.bot/spec/mmp-consent](https://sym.bot/spec/mmp-consent) · Protocol version: `1`

## State machine (§4)

```
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

Invariants enforced by `invariants::check_transition()`:
- WITHDRAWN → any = **hard reject**
- SUSPENDED → SUSPENDED = idempotent (no-op)
- GRANTED → GRANTED (via resume) = idempotent (no-op)

## Zero-allocation guarantee

Default build (`#![no_std]`, no features) is **allocation-free**. Frame types use `ReasonBuf` (64-byte fixed buffer). Encoder writes to caller-provided `&mut [u8]` and returns `Result<usize, EncodeError>`.

## Threat model

| Threat | Attack vector | Mitigation |
|--------|--------------|------------|
| Map bomb | CBOR map with 10⁶ entries → CPU exhaustion | `MAX_MAP_FIELDS = 8` — reject on decode |
| String bomb | 1 MB text string → memory exhaustion | `MAX_STRING_LEN = 128` — reject on decode |
| Recursive nesting | Deeply nested maps/arrays → stack overflow | `MAX_NESTING_DEPTH = 4` — reject on skip |
| Type confusion | `{"type":"withdraw","type":"resume"}` → last wins | Bitmask duplicate detection → reject on second hit |
| Unsupported CBOR | Negative ints, tags, floats → undefined behavior | Explicit rejection of major types 1,2,4,6,7 |
| Buffer overflow | Large frame exceeds encode buffer | `encode()` returns `Err(BufferTooSmall)` |
| State violation | Resume from WITHDRAWN | `check_transition()` returns `Err(TransitionFromWithdrawn)` |

## Determinism analysis

```
Decoder: O(n), n ≤ MAX_MAP_FIELDS = 8
  Per field: 1 key read (bounded) + 1 value read (bounded) = O(1)
  Total: ≤ 16 CBOR item reads, each O(1)
  No loops beyond field count. No recursion in normal path.
  skip_value: bounded by MAX_NESTING_DEPTH × MAX_MAP_FIELDS

Encoder: O(k), k = present optional fields ≤ 7
  Per field: key write + value write = O(1)
  No allocation. No branching beyond field presence.

State machine: O(1) — single match on (current_state, frame_type)

WCET target: <10µs decode, <5µs encode on Cortex-M4F @ 168 MHz
```

## Spec-to-code mapping

| Spec § | Module | Requirement | Enforcement |
|--------|--------|-------------|-------------|
| §3 | `frames` | Frame types | Type-safe enum |
| §3.1 | `frames::ConsentWithdraw` | scope MUST be present | Non-optional field |
| §3.1 | `invariants` | timestamp SHOULD be present | Warning if absent |
| §3.4 | `reason` | Reason code registry | `ReasonCode` enum with ranges |
| §4 | `state` | State transitions | `ConsentState::withdraw/suspend/resume` |
| §4 | `invariants::check_transition` | WITHDRAWN is terminal | Hard reject |
| §5.1 | `engine::withdraw` | Local enforcement before notification | Steps 1-4 in Secure World |
| §6.1 | `engine::allows_cognitive_frames` | Filter during SUSPENDED | Returns `false` |
| §6.4 | `state::to_gossip_bits` | 2-bit compact encoding | `00/01/10` |
| §7 | `codec::cbor` | Wire format | String-keyed CBOR map |
| §7 | `codec::cbor::skip_value` | Forward compatibility | Skip unknown keys |
| §8 | `stim_guard` | BCI hardware enforcement | DAC gate trait |
| §10 | `invariants` | MUST/SHOULD/MAY conformance | Violations vs warnings |

## Security bounds

| Constant | Value | Purpose |
|----------|-------|---------|
| `MAX_MAP_FIELDS` | 8 | Map bomb protection |
| `MAX_STRING_LEN` | 128 | String bomb protection |
| `MAX_NESTING_DEPTH` | 4 | Stack protection |
| `MAX_REASON_LEN` | 64 | ReasonBuf capacity |
| `MAX_ENCODED_SIZE` | 256 | Encoder buffer ceiling |
| `MAX_PEERS` | 8 | Engine peer table |
| `CONSENT_PROTOCOL_VERSION` | 1 | Wire versioning |

## Duplicate key detection

Bitmask over known keys (documented in `codec::cbor::decode`):

```
bit 0 (0x01) = "type"          §3
bit 1 (0x02) = "scope"         §3.1
bit 2 (0x04) = "reasonCode"    §3.4
bit 3 (0x08) = "reason"        §3.1
bit 4 (0x10) = "epoch"         §3.1
bit 5 (0x20) = "timestamp"     §3.1
bit 6 (0x40) = "timestamp_us"  §3.1 (AxonOS extension)
```

Second occurrence of any known key → `Err(DuplicateKey)`.

## Fuzzing

```bash
cargo +nightly fuzz run fuzz_cbor_decode     # crash resistance
cargo +nightly fuzz run fuzz_cbor_roundtrip  # encode→decode invariant
```

Targets: `decode()`, `skip_value()`, `read_text_bounded()`.

## Test suite

```bash
cargo test                  # no_std: CBOR round-trip, security, state machine, engine, invariants
cargo test --features json  # + JSON round-trip against all 15 interop vectors
```

40+ tests covering: 7 CBOR round-trips, 5 security rejection tests, 2 buffer overflow tests, 6 invariant tests, 15 vector round-trips, 5 state machine tests, 5 engine tests.

## CI

GitHub Actions: `cargo test`, `cargo test --features json`, `cargo build --target thumbv7em-none-eabihf`, `clippy -D warnings`, `rustfmt --check`.

## Licence

MIT

## Links

[axonos.org](https://axonos.org) · [medium.com/@AxonOS](https://medium.com/@AxonOS) · axonosorg@gmail.com
[sym.bot/spec/mmp](https://sym.bot/spec/mmp) · [github.com/sym-bot](https://github.com/sym-bot)
