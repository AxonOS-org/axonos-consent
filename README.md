# axonos-consent

[![version](https://img.shields.io/badge/version-0.2.2-blue)](https://github.com/AxonOS-org/axonos-consent/releases)
![MMP](https://img.shields.io/badge/MMP-v0.2.3-purple)
![consent-ext](https://img.shields.io/badge/consent--ext-v0.1.0-green)
[![no\_std](https://img.shields.io/badge/no__std-%E2%9C%93-brightgreen)](#guarantees)
[![unsafe](https://img.shields.io/badge/unsafe-forbidden-red)](src/lib.rs)
[![alloc](https://img.shields.io/badge/alloc-zero-orange)](#guarantees)
[![tests](https://img.shields.io/badge/tests-unit%20%2B%20interop%20%2B%20fuzz-blue)](#testing)
[![fuzz](https://img.shields.io/badge/fuzz-%E2%9C%93-yellow)](#testing)
[![license](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue)](#licence)

**The Rust reference implementation of the MMP Consent Extension v0.1.0.**

The specification is authored by SYM.BOT. This crate is the Rust implementation of that specification — `#![no_std]`, zero allocation, bounded WCET, hardware-gated stimulation interlock. Targets the hard real-time execution profile required for safety-critical brain-computer interface applications.

---

## Specification attribution

This crate implements protocol specifications authored by SYM.BOT. The specifications are the normative source; this crate is one possible implementation of them.

- **Mesh Memory Protocol (MMP) v0.2.3** — authored by SYM.BOT, published at sym.bot/spec/mmp, licensed under CC-BY-4.0.
- **MMP Consent Extension v0.1.0** — specification authored by SYM.BOT, licensed under CC-BY-4.0.
- **Symbolic-Vector Attention Fusion (SVAF)** — authored by Hongwei Xu, [arXiv:2604.03955](https://arxiv.org/abs/2604.03955).

This Rust implementation is independent work by AxonOS. SYM.BOT did not author, co-author, or contribute to the Rust source code in this repository. Conversely, AxonOS did not author, co-author, or contribute to the specifications listed above.

The `axonos-consent` crate (this repository) is dual-licensed Apache-2.0 OR MIT.

---

## Architecture

Consent operates at **Layer 2** (Connection) of the MMP 8-layer stack — below messaging, below SVAF coupling (Layer 4), below cognition (Layers 5–7). Per MMP Consent Extension §4.1, a `consent-withdraw` frame closes the gate before any higher-layer logic executes.

```text
┌─────────────────────────────────────────────────┐
│  Non-Secure World (application core)            │
│  ┌──────────────────────────────────────────┐   │
│  │  Network task                            │   │
│  │  ├─ JSON codec (relay boundary)          │   │
│  │  └─ Frame parser ↔ sym relay (MMP §7)    │   │
│  └──────────────┬───────────────────────────┘   │
│                 │ nsc_withdraw_consent()         │
├─────────────────┼───────────────────────────────┤
│  Secure World   │ (TrustZone, target platform)  │
│                 ▼                                │
│  ┌──────────────────────────────────────────┐   │
│  │  ConsentEngine (this crate)              │   │
│  │  wire → decode → validate → transition   │   │
│  │  → StimGuard → Secure GPIO DAC gate      │   │
│  └──────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

**Why Layer 2.** Per the specification, consent operating at Layer 4 would mean the SVAF coupling engine could delay or deprioritise a withdrawal request. The specification's choice to place consent at Layer 2 eliminates this class of failure; this crate is the execution layer where that choice is physically enforced on hardware.

---

## Conformance

This implementation passes the 15 canonical interop test vectors for MMP Consent Extension v0.1.0. The vector set, authored by SYM.BOT as part of the specification, is reproduced in [`tests/vectors/consent-interop-vectors-v0.1.0.json`](tests/vectors/consent-interop-vectors-v0.1.0.json).

### Conformance status (this implementation only)

| Specification | This implementation | Vectors | Result |
|:---:|:---|:---:|:---:|
| MMP Consent Extension v0.1.0 | `axonos-consent` v0.2.2 | 15/15 | **PASS** |
| MMP v0.2.3 | `axonos-consent` v0.2.2 | 15/15 | Backward-compatible |

v0.2.2 is additive (audit trail, API ergonomics). State machine and wire format unchanged from the v0.1.0 vectors.

Other implementations of the MMP Consent Extension exist; their conformance status is not represented by this repository. Refer to the specification or to each respective project for their own conformance claims.

### Integrity lock

```
SHA-256: 29a8bf9f2b4dabe5d9641a8a4c416f361c2ba9815cca9b8e9e1d222d002fa50a
```

Any modification to [`tests/vectors/consent-interop-vectors-v0.1.0.json`](tests/vectors/consent-interop-vectors-v0.1.0.json) invalidates the bundled vector file. [CI verifies](https://github.com/AxonOS-org/axonos-consent/actions/workflows/ci.yml) this checksum on every push.

---

## API

```rust
use axonos_consent::ConsentEngine;

let mut engine = ConsentEngine::new();
engine.register_peer(peer_id, now_us).unwrap();

// Single entry point: wire bytes → validated state transition
let result = engine.process_raw(&peer_id, cbor_bytes, now_us)?;
// result.new_state : ConsentState
// result.warnings  : SHOULD-level advisories (RFC 2119)
```

Full runtime pipeline — no other function combination needed:

```text
process_raw  →  CBOR decode (bounded)
             →  invariant check (MUST/SHOULD per specification §10)
             →  state transition (exhaustive 3×3 FSM per §4)
             →  StimGuard hardware interlock
```

---

## State machine

Implements the specification's §4 state machine. Exhaustive 3×3 match on `(ConsentState, ConsentFrame)`. Zero wildcard arms — adding a new state or frame variant produces a **compile error**.

```text
 ┌─────────┐  consent-suspend   ┌───────────┐
 │ GRANTED │ ─────────────────→ │ SUSPENDED │
 │         │ ←───────────────── │           │
 └────┬────┘  consent-resume    └─────┬─────┘
      │                               │
      │  consent-withdraw             │  consent-withdraw
      ▼                               ▼
 ┌──────────────────────────────────────┐
 │          WITHDRAWN (terminal)         │
 └──────────────────────────────────────┘
```

| | `Withdraw` | `Suspend` | `Resume` |
|:---|:---:|:---:|:---:|
| **GRANTED** | → WITHDRAWN | → SUSPENDED | → GRANTED *(idempotent)* |
| **SUSPENDED** | → WITHDRAWN | → SUSPENDED *(idempotent)* | → GRANTED |
| **WITHDRAWN** | **REJECT** | **REJECT** | **REJECT** |

See [`state.rs`](src/state.rs) — `apply_frame()`.

---

## Guarantees (this implementation)

| Property | Guarantee | Enforcement |
|:---|:---|:---|
| `#![no_std]` | Default build, no heap | [`Cargo.toml`](Cargo.toml) |
| Zero-allocation | `ReasonBuf` 64 B fixed, encoder writes to `&mut [u8]` | [`frames.rs`](src/frames.rs) |
| Bounded parsing | `MAX_MAP=8` · `MAX_STR=128` · `MAX_DEPTH=4` | [`cbor.rs`](src/codec/cbor.rs) |
| No unsafe | `#![forbid(unsafe_code)]` — compile-time | [`lib.rs`](src/lib.rs) |
| Exhaustive FSM | 3×3 table, compiler-checked, no wildcards | [`state.rs`](src/state.rs) |
| Deterministic | O(1) transitions, O(n ≤ 8) decode | [`engine.rs`](src/engine.rs) |
| `WITHDRAWN` terminal | Any frame after `WITHDRAWN` → REJECT | [`state.rs`](src/state.rs) |

These are properties of this Rust implementation. The specification at sym.bot/spec/mmp defines the required behaviour; the properties above describe how this implementation achieves it.

---

## Threat model (this implementation)

| Threat | Mitigation | Bound | Source |
|:---|:---|:---:|:---|
| Map bomb | `MAX_MAP_FIELDS` | 8 | [`cbor.rs`](src/codec/cbor.rs) |
| String bomb | `MAX_STRING_LEN` | 128 B | [`cbor.rs`](src/codec/cbor.rs) |
| Stack overflow | `MAX_NESTING_DEPTH` | 4 | [`cbor.rs`](src/codec/cbor.rs) |
| Type confusion | Bitmask duplicate key detection | 7 keys | [`cbor.rs`](src/codec/cbor.rs) |
| Unsupported CBOR | Explicit reject: types 1, 2, 4, 6, 7 | [RFC 8949](https://www.rfc-editor.org/rfc/rfc8949) | [`cbor.rs`](src/codec/cbor.rs) |
| Buffer overflow | `Err(BufferTooSmall)` | 256 B | [`cbor.rs`](src/codec/cbor.rs) |
| State violation | `apply_frame()` REJECT | Compile-time exhaustiveness | [`state.rs`](src/state.rs) |

---

## Error taxonomy

```text
L1 (Wire)     → Error::Decode     — malformed CBOR, bounds, unsupported types
L2 (Struct)   → Error::Invariant  — MUST violations (specification §10)
L3 (State)    → Error::Transition — WITHDRAWN → any, peer not found
L4 (System)   → Error::Encode     — buffer too small
```

`#[must_use]` on `Error` enum. `From` conversions for each layer. See [`error.rs`](src/error.rs).

---

## WCET (this implementation, target Cortex-M4F)

Per-operation worst-case timing on Cortex-M4F @ 168 MHz (instruction-count derived; [`engine.rs`](src/engine.rs)). GPIO-validated oscilloscope measurement on Cortex-M33 is pending.

| Operation | WCET† | Complexity |
|:---|:---:|:---|
| `process_raw` (full pipeline incl. CBOR decode) | <10 µs | O(n), n ≤ 8 fields |
| `process_frame` (already-decoded path) | <1 µs | O(1) |
| `withdraw_all` (emergency global kill, 8 peers) | <5 µs | O(MAX_PEERS) |

† Instruction-count derived on Cortex-M4F. GPIO-validated measurement is pending. Figures characterize this Rust implementation only and are not a specification claim.

---

## MMP v0.2.3 alignment (this implementation)

This crate targets the sections of the MMP base specification listed below.

| MMP § | This implementation's alignment |
|:---|:---|
| §3.5 | `consent-withdraw` triggers CONNECTED → DISCONNECTED |
| §7 | Forward compat: unknown frame types silently ignored |
| §7.2 | Error code `2002 CONSENT_WITHDRAWN` |
| §16 | Extension mechanism: `consent-v0.1.0` in handshake |
| §16.4 | Published extension: sym.bot/spec/mmp |

---

## Reason codes

Implements the registry defined in MMP Consent Extension §3.4. Spec-defined codes (`0x00`–`0x0F`) are reserved by the specification; `0x10`–`0xFF` are implementation-specific.

| Code | Name | Range |
|:---:|:---|:---:|
| `0x00` | `UNSPECIFIED` | spec |
| `0x01` | `USER_INITIATED` | spec |
| `0x02` | `SAFETY_VIOLATION` | spec |
| `0x03` | `HARDWARE_FAULT` | spec |
| `0x10` | `STIMGUARD_LOCKOUT` | AxonOS |
| `0x11` | `SESSION_ATTESTATION_FAILURE` | AxonOS |
| `0x12` | `EMERGENCY_BUTTON` | AxonOS |
| `0x13` | `SWARM_FAULT_DETECTED` | AxonOS |

Unknown codes → `UNSPECIFIED` (forward-compatible per specification §3.4). See [`reason.rs`](src/reason.rs).

---

## Regulatory context

The implementation is designed to be usable as a component in systems pursuing the following frameworks. Qualification against these frameworks is the responsibility of the downstream integrator.

| Framework | Relevance | Reference |
|:---|:---|:---|
| [IEC 62304](https://www.iso.org/standard/71604.html) Class C | Medical device software lifecycle | Architecture aligned |
| [IEC 60601-1](https://www.iso.org/standard/65529.html) | Essential performance, basic safety | StimGuard interlock |
| [ISO 14971](https://www.iso.org/standard/72704.html) | Risk management for medical devices | [Threat model](#threat-model-this-implementation) |
| [Shannon criteria](https://doi.org/10.1109/10.126616) | Charge density limits (k=1.75, ≤ 30 µC/cm²) | [`stim_guard.rs`](src/stim_guard.rs) |

---

## Crate structure

```text
src/
├── lib.rs             # #![forbid(unsafe_code)], spec-to-code mapping
├── state.rs           # ConsentState + apply_frame (exhaustive 3×3)
├── engine.rs          # ConsentEngine, process_raw, process_frame
├── frames.rs          # Frame types, ReasonBuf (64 B zero-alloc)
├── reason.rs          # ReasonCode registry (§3.4)
├── invariants.rs      # MUST/SHOULD/MAY enforcement (§10)
├── error.rs           # Layered error taxonomy (L1–L4)
├── stim_guard.rs      # DacGate trait, timing contract (§8)
└── codec/
    ├── cbor.rs        # Bounded encoder/decoder, security hardened
    └── json.rs        # JSON codec (feature-gated)
tests/
├── consent_interop.rs
└── vectors/
    └── consent-interop-vectors-v0.1.0.json  # per MMP Consent Extension §11
fuzz/
└── fuzz_targets/
    ├── fuzz_cbor_decode.rs
    └── fuzz_cbor_roundtrip.rs
```

---

## Embedded

```bash
cargo build --release --target thumbv7em-none-eabihf --no-default-features
```

Zero runtime dependencies in `no_std` mode. No OS required. Bare-metal Cortex-M4F ready.

---

## Testing

```bash
cargo test                     # Unit + integration (no_std default)
cargo test --features json     # + JSON round-trip (15 canonical vectors)
cargo +nightly fuzz run fuzz_cbor_decode      # Crash resistance
cargo +nightly fuzz run fuzz_cbor_roundtrip   # encode→decode invariant
```

[CI pipeline](https://github.com/AxonOS-org/axonos-consent/actions/workflows/ci.yml): `test` · `test --features json` · `build thumbv7em` · `clippy -D warnings` · `fmt --check` · SHA-256 vector integrity.

---

## Licence

The `axonos-consent` Rust source code in this repository is dual-licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.

The MMP and MMP Consent Extension specifications that this crate implements are authored and licensed separately by SYM.BOT under CC-BY-4.0 — see sym.bot/spec/mmp for their terms.

---

## References

- Xu, H. (2026). *Mesh Memory Protocol v0.2.3*. SYM.BOT Ltd. sym.bot/spec/mmp. CC-BY-4.0.
- Xu, H. (2026). *MMP Consent Extension v0.1.0*. SYM.BOT Ltd. sym.bot/spec/mmp. CC-BY-4.0.
- Xu, H. (2026). *Symbolic-Vector Attention Fusion for Collective Intelligence*. [arXiv:2604.03955](https://arxiv.org/abs/2604.03955).

---

**AxonOS** · [axonos.org](https://axonos.org) · [medium.com/@AxonOS](https://medium.com/@AxonOS) · [linkedin.com/in/axonos](https://www.linkedin.com/in/axonos) · axonosorg@gmail.com
