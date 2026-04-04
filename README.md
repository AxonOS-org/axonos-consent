# axonos-consent

[![CI](https://github.com/AxonOS-org/axonos-consent/actions/workflows/ci.yml/badge.svg)](https://github.com/AxonOS-org/axonos-consent/actions/workflows/ci.yml)

**MMP Consent Extension v0.1.0 — zero-alloc Rust implementation for real-time BCI.**

Spec: [sym.bot/spec/mmp-consent](https://sym.bot/spec/mmp-consent) · Base: [MMP v0.2.0](https://sym.bot/spec/mmp) · Ref impls: [sym](https://github.com/sym-bot/sym) (Node.js) · [sym-swift](https://github.com/sym-bot/sym-swift) (iOS)

## Zero-allocation guarantee

Default build (`#![no_std]`, no features) is **fully allocation-free**. No `String`, no `Vec`, no heap. All frame types use fixed-size buffers (`ReasonBuf`: 64 bytes). Safe for Cortex-M4F critical path.

The `alloc` feature enables heap-backed types for relay boundary use. The `json` feature enables JSON codec (requires `alloc` + `std`).

## Security bounds

| Limit | Value | Protects against |
|-------|-------|-----------------|
| `MAX_MAP_FIELDS` | 8 | Map bomb / DoS |
| `MAX_STRING_LEN` | 128 bytes | Unbounded allocation |
| `MAX_NESTING_DEPTH` | 4 | Stack overflow via crafted CBOR |
| Duplicate key detection | bitmask | Type confusion attack |

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Non-Secure World (A53)                         │
│  ┌──────────────────────────────────────────┐   │
│  │  Network Task                            │   │
│  │  ├─ JSON codec (relay boundary)          │   │
│  │  └─ Frame parser ↔ sym relay             │   │
│  └──────────────┬───────────────────────────┘   │
│                 │ nsc_withdraw_consent()         │
├─────────────────┼───────────────────────────────┤
│  Secure World   │ (TrustZone-S)                 │
│                 ▼                                │
│  ┌──────────────────────────────────────────┐   │
│  │  ConsentEngine (zero-alloc)              │   │
│  │  ├─ Per-peer state machine (8 slots)     │   │
│  │  ├─ CBOR codec (bounded decoder)         │   │
│  │  ├─ Validation layer                     │   │
│  │  └─ StimGuard → Secure GPIO DAC gate     │   │
│  └──────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| *(none)* | ✅ | `#![no_std]`, zero-alloc, CBOR codec |
| `alloc` | ❌ | Heap types for relay boundary |
| `json` | ❌ | JSON codec (requires `alloc` + `std`) |
| `stim-guard` | ❌ | StimGuard hardware integration |

## Crate structure

```
src/
├── lib.rs           # crate root, feature gates
├── state.rs         # ConsentState enum + transitions
├── engine.rs        # ConsentEngine — per-peer state machine
├── frames.rs        # Frame types with ReasonBuf (zero-alloc)
├── reason.rs        # ReasonCode registry
├── validate.rs      # Structural validation layer
├── stim_guard.rs    # DacGate trait + StimGuard (feature-gated)
└── codec/
    ├── cbor.rs      # Security-bounded CBOR encoder/decoder
    └── json.rs      # JSON encoder/decoder (feature-gated)
tests/
├── consent_interop.rs               # 40+ tests: round-trip, security, engine
└── vectors/
    └── consent-interop-vectors-v0.1.0.json  # 15 interop test vectors
```

## Test vectors

15 cases covering all frame types, scopes, reason codes, edge cases (idempotent ops, unknown codes, timestamp precedence), gossip encoding, and BCI-specific scenarios (StimGuard lockout, emergency button).

```bash
cargo test                  # no_std tests (CBOR, state machine, engine)
cargo test --features json  # + JSON round-trip against all 15 vectors
```

## Reason code registry

| Code | Name | Range |
|------|------|-------|
| 0x00 | UNSPECIFIED | spec |
| 0x01 | USER_INITIATED | spec |
| 0x02 | SAFETY_VIOLATION | spec |
| 0x03 | HARDWARE_FAULT | spec |
| **0x10** | **STIMGUARD_LOCKOUT** | AxonOS |
| 0x11 | SESSION_ATTESTATION_FAILURE | AxonOS |
| 0x12 | EMERGENCY_BUTTON | AxonOS |
| 0x13 | SWARM_FAULT_DETECTED | AxonOS |

## Licence

MIT

## Links

[axonos.org](https://axonos.org) · [medium.com/@AxonOS](https://medium.com/@AxonOS) · [github.com/AxonOS-org](https://github.com/AxonOS-org) · axonosorg@gmail.com
