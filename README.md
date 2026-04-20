# axonos-consent

[![version](https://img.shields.io/badge/version-0.2.2-blue)](https://github.com/AxonOS-org/axonos-consent/releases)
[![MMP](https://img.shields.io/badge/MMP-v0.2.2-purple)](https://sym.bot/spec/mmp)
[![consent-ext](https://img.shields.io/badge/consent--ext-v0.1.0-green)](https://sym.bot/spec/mmp-consent)
[![no\_std](https://img.shields.io/badge/no__std-%E2%9C%93-brightgreen)](#guarantees)
[![unsafe](https://img.shields.io/badge/unsafe-forbidden-red)](src/lib.rs)
[![alloc](https://img.shields.io/badge/alloc-zero-orange)](#guarantees)
[![interop](https://img.shields.io/badge/interop-15%2F15-success)](#interoperability)
[![tests](https://img.shields.io/badge/tests-unit%20%2B%20interop%20%2B%20fuzz-blue)](#testing)
[![fuzz](https://img.shields.io/badge/fuzz-%E2%9C%93-yellow)](#testing)
[![SVAF](https://img.shields.io/badge/SVAF-arXiv%3A2604.03955-b31b1b)](https://arxiv.org/abs/2604.03955)
[![license](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue)](#licence)
[![CI](https://github.com/AxonOS-org/axonos-consent/actions/workflows/ci.yml/badge.svg)](https://github.com/AxonOS-org/axonos-consent/actions/workflows/ci.yml)

**The reference hard real-time execution environment for the MMP Consent Extension.**

Deterministic consent enforcement for safety-critical brain-computer interfaces. `#![no_std]` Rust, bare-metal Cortex-M, zero allocation, bounded WCET, hardware-gated stimulation interlock.

Aligned with [Mesh Memory Protocol v0.2.2](https://sym.bot/spec/mmp) §16.4, implements [MMP Consent Extension v0.1.0](https://sym.bot/spec/mmp-consent) co-designed with [Hongwei Xu](https://github.com/sym-bot) ([SYM.BOT](https://sym.bot)).

Joint paper: *"Protocol-Level Consent for Cognitive Mesh Coupling"* — built on [arXiv:2604.03955](https://arxiv.org/abs/2604.03955) (SVAF).

> *"The consent primitive was designed together — your BCI domain constraints shaped the spec."*
> — [Hongwei Xu](https://sym.bot), Founder of SYM.BOT

---

## Layer positioning

The MMP Consent Extension is a **protocol specification**. It defines frame types, a per-peer state machine, and conformance vectors. It says nothing about **how** implementations execute — that is left to each execution environment.

```text
Protocol layer    MMP Consent Extension v0.1.0
                  ─ defines state transitions, frames, reason codes
                  ─ implementation-agnostic, runtime-agnostic

Execution layer   axonos-consent — Rust #![no_std], bare-metal,
                  below the protocol boundary
                  ─ enforces transitions in hard real-time
                  ─ gates hardware stimulation via Secure World GPIO
                  ─ bounded WCET; no heap, no GC, no async
```

**Protocols define state transitions. `axonos-consent` is the runtime that enforces them under real-time constraints.** Other implementations exist — the Node.js reference in [`sym`](https://github.com/sym-bot/sym) is the relay-side client — but they target different execution profiles (soft real-time, general-purpose runtime) and do not claim hard WCET bounds or hardware-gated enforcement.

---

## Architecture

Consent operates at **Layer 2** (Connection) of the [MMP 8-layer stack](https://sym.bot/spec/mmp) — below messaging, below [SVAF coupling](https://arxiv.org/abs/2604.03955) (Layer 4), below cognition (Layers 5–7). A consent-withdraw frame closes the gate before any higher-layer logic executes.

```text
┌─────────────────────────────────────────────────┐
│  Non-Secure World (A53 / application core)      │
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

**Why Layer 2.** If consent operated at Layer 4, the [SVAF](https://arxiv.org/abs/2604.03955) coupling engine could delay or deprioritise a withdrawal request. Consent at Layer 2 eliminates this class of failure in any compliant implementation — AxonOS is where the guarantee is physically enforced on hardware.

---

## Interoperability

The MMP Consent Extension specification has two implementations as of April 2026, each targeting a different execution profile.

### Compatibility matrix

| Spec version | Implementation | Role | Execution profile | Vectors | Result |
|:---:|:---|:---|:---|:---:|:---:|
| [Consent v0.1.0](https://sym.bot/spec/mmp-consent) | **axonos-consent v0.2.2** (this crate) | Reference hard real-time runtime | Rust `#![no_std]`, bare-metal Cortex-M, bounded WCET | **15/15** | **PASS** |
| [Consent v0.1.0](https://sym.bot/spec/mmp-consent) | [sym](https://github.com/sym-bot/sym) (Node.js) | Reference relay-side client | Node.js V8, soft real-time | 15/15 | PASS |
| [MMP v0.2.2](https://sym.bot/spec/mmp) | axonos-consent v0.2.2 | — | — | 15/15 | Backward-compatible |

> **Interop verification across execution environments.** Both implementations pass the same 15 canonical vectors and produce identical state transitions for every input. Only `axonos-consent` targets the hard real-time + hardware-interlock profile required for BCI stimulation-gating deployments; the Node.js reference targets relay-side general-purpose deployments.

**v0.2.2 note:** additive release (audit trail, API ergonomics). State machine and wire format unchanged from v0.1.0.

### Two-implementation audit trail

Design independence follows the [§6.1.2 qualified-independence methodology](https://sym.bot/spec/mmp-consent): `axonos-consent` (Rust) was the first implementation; [`sym`](https://github.com/sym-bot/sym) (Node.js) was built after a line-by-line audit of the Rust source, with engineering decisions appropriate to the Node.js runtime. Five documented divergences in error handling, failure policy, transport encoding, peer-table representation, and reason-code normalization demonstrate this is not a transliteration — same specification, independent engineering realizations.

### Field validation

Validated against [SYM.BOT](https://sym.bot) production mesh (5 active nodes, April 2026). Four consent frames (`withdraw`, `suspend`, `resume`, `STIMGUARD_LOCKOUT`) forwarded by relay; non-consent-enabled production nodes silently ignored them per [MMP §7](https://sym.bot/spec/mmp) forward compatibility. Zero transport errors. This validates MMP §7 forward-compatibility behaviour; consent-semantics interoperability is established by the 15/15 vector conformance.

### Integrity lock

```
SHA-256: 29a8bf9f2b4dabe5d9641a8a4c416f361c2ba9815cca9b8e9e1d222d002fa50a
```

Any modification to [`tests/vectors/consent-interop-vectors-v0.1.0.json`](tests/vectors/consent-interop-vectors-v0.1.0.json) invalidates the test suite. [CI verifies](https://github.com/AxonOS-org/axonos-consent/actions/workflows/ci.yml) this checksum on every push.

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
             →  invariant check (MUST/SHOULD, RFC 2119)
             →  state transition (exhaustive 3×3 FSM)
             →  StimGuard hardware interlock
```

---

## State machine

Per [Consent Extension §4](https://sym.bot/spec/mmp-consent). Exhaustive 3×3 match on `(ConsentState, ConsentFrame)`. Zero wildcard arms — adding a new state or frame variant produces a **compile error**.

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

## Guarantees

| Property | Guarantee | Enforcement |
|:---|:---|:---|
| `#![no_std]` | Default build, no heap | [`Cargo.toml`](Cargo.toml) |
| Zero-allocation | `ReasonBuf` 64 B fixed, encoder writes to `&mut [u8]` | [`frames.rs`](src/frames.rs) |
| Bounded parsing | `MAX_MAP=8` · `MAX_STR=128` · `MAX_DEPTH=4` | [`cbor.rs`](src/codec/cbor.rs) |
| No unsafe | `#![forbid(unsafe_code)]` — compile-time | [`lib.rs`](src/lib.rs) |
| Exhaustive FSM | 3×3 table, compiler-checked, no wildcards | [`state.rs`](src/state.rs) |
| Deterministic | O(1) transitions, O(n ≤ 8) decode | [`engine.rs`](src/engine.rs) |
| `WITHDRAWN` terminal | Any frame after `WITHDRAWN` → REJECT | [`state.rs`](src/state.rs) |
| Layer 2 | Below coupling (Layer 4), below [SVAF](https://arxiv.org/abs/2604.03955) | [MMP §16.4](https://sym.bot/spec/mmp) |

---

## Threat model

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
L2 (Struct)   → Error::Invariant  — MUST violations (§10)
L3 (State)    → Error::Transition — WITHDRAWN → any, peer not found
L4 (System)   → Error::Encode     — buffer too small
```

`#[must_use]` on `Error` enum. `From` conversions for each layer. See [`error.rs`](src/error.rs).

---

## WCET analysis

Per-operation worst-case timing on Cortex-M4F @ 168 MHz (instruction-count derived; [`engine.rs`](src/engine.rs)). GPIO-validated oscilloscope measurement on Cortex-M33 is the next milestone.

| Operation | WCET† | Complexity |
|:---|:---:|:---|
| `process_raw` (full pipeline incl. CBOR decode) | <10 µs | O(n), n ≤ 8 fields |
| `process_frame` (already-decoded path) | <1 µs | O(1) |
| `withdraw_all` (emergency global kill, 8 peers) | <5 µs | O(MAX_PEERS) |
| §5.1 steps 3–5 (emergency button → DAC gate) | <1 µs | O(1), non-preemptible |

† Instruction-count derived on Cortex-M4F; GPIO-validated measurement on Cortex-M33 is the next milestone. All figures are per-operation — the joint paper uses per-operation attribution, not a collapsed headline.

---

## MMP v0.2.2 alignment

| [MMP](https://sym.bot/spec/mmp) § | Alignment |
|:---|:---|
| §3.5 | `consent-withdraw` triggers CONNECTED → DISCONNECTED |
| §7 | Forward compat: unknown frame types silently ignored |
| §7.2 | Error code `2002 CONSENT_WITHDRAWN` |
| §16 | Extension mechanism: `consent-v0.1.0` in handshake |
| §16.4 | Published: [`sym.bot/spec/mmp-consent`](https://sym.bot/spec/mmp-consent) |

---

## Reason codes

Per [Consent Extension §3.4](https://sym.bot/spec/mmp-consent):

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

Unknown codes → `UNSPECIFIED` (forward-compatible per §3.4). See [`reason.rs`](src/reason.rs).

---

## Regulatory alignment

| Framework | Relevance | Reference |
|:---|:---|:---|
| [IEC 62304](https://www.iso.org/standard/71604.html) Class C | Medical device software lifecycle | Architecture aligned |
| [IEC 60601-1](https://www.iso.org/standard/65529.html) | Essential performance, basic safety | StimGuard enforcement |
| [ISO 14971](https://www.iso.org/standard/72704.html) | Risk management for medical devices | [Threat model](#threat-model) |
| [UNESCO 2025](https://www.unesco.org/en/articles/ethics-neurotechnology) | Ethics of Neurotechnology — consent sovereignty | "at any time" withdrawal right |
| [Shannon criteria](https://doi.org/10.1109/10.126616) | Charge density limits (k=1.75, ≤ 30 µC/cm²) | [`stim_guard.rs`](src/stim_guard.rs) |
| [FDA BCI Guidance](https://www.fda.gov/regulatory-information/search-fda-guidance-documents/implanted-brain-computer-interface-bci-devices-patients-paralysis-or-amputation-non-clinical-testing) | Implanted BCI non-clinical testing (2021) | Cybersecurity + safety |

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
    └── consent-interop-vectors-v0.1.0.json
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

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE) ([http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))
- [MIT License](LICENSE-MIT) ([http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))

at your option.

---

**AxonOS** · [axonos.org](https://axonos.org) · [medium.com/@AxonOS](https://medium.com/@AxonOS) · [github.com/AxonOS-org](https://github.com/AxonOS-org) · axonosorg@gmail.com

**SYM.BOT** · [sym.bot](https://sym.bot) · [sym.bot/spec/mmp](https://sym.bot/spec/mmp) · [sym.bot/spec/mmp-consent](https://sym.bot/spec/mmp-consent) · [github.com/sym-bot](https://github.com/sym-bot)

**Papers** · [arXiv:2604.03955](https://arxiv.org/abs/2604.03955) (SVAF) · *Protocol-Level Consent for Cognitive Mesh Coupling* (in preparation)
