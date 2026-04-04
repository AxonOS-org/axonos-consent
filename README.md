# axonos-consent

**MMP Consent Extension v0.2.0 — Rust implementation for AxonOS**

Implementation of the [MMP Consent Extension](https://sym.bot/spec/mmp-consent) for real-time brain-computer interface applications. Part of the [AxonOS](https://axonos.org) neural operating system.

## Specification

- **Consent Extension**: [sym.bot/spec/mmp-consent](https://sym.bot/spec/mmp-consent) (CC BY 4.0)
- **Base Protocol**: [MMP v0.2.0](https://sym.bot/spec/mmp) — Mesh Memory Protocol by [SYM.BOT Ltd](https://sym.bot)
- **Reference Implementations**: [sym](https://github.com/sym-bot/sym) (Node.js) · [sym-swift](https://github.com/sym-bot/sym-swift) (iOS/macOS)

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Non-Secure World (A53)                         │
│                                                 │
│  ┌──────────────────────────────────────────┐   │
│  │  Network Task                            │   │
│  │  ├─ JSON codec (relay boundary)          │   │
│  │  └─ Frame parser ←→ sym relay            │   │
│  └──────────────┬───────────────────────────┘   │
│                 │ nsc_withdraw_consent()         │
├─────────────────┼───────────────────────────────┤
│  Secure World   │ (TrustZone-S)                 │
│                 ▼                                │
│  ┌──────────────────────────────────────────┐   │
│  │  ConsentEngine                           │   │
│  │  ├─ Per-peer state machine               │   │
│  │  ├─ CBOR codec (local IPC)               │   │
│  │  └─ StimGuard integration                │   │
│  │      └─ Secure GPIO DAC gate close       │   │
│  └──────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

## Dual Codec Strategy

| Context | Encoding | Rationale |
|---------|----------|-----------|
| Local IPC (M4F ↔ A53) | **CBOR** | Compact, no string parsing on critical path |
| Relay boundary (A53 ↔ mesh) | **JSON** | Compatible with MMP reference implementations |

Frame types use **string identifiers** per MMP Section 7:
`"consent-withdraw"`, `"consent-suspend"`, `"consent-resume"`

## Crate Structure

```
axonos-consent/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs              # Crate root, feature gates
│   ├── state.rs            # ConsentState enum + transitions + gossip encoding
│   ├── engine.rs           # ConsentEngine — per-peer state machine
│   ├── frames.rs           # Frame types: Withdraw, Suspend, Resume
│   ├── reason.rs           # ReasonCode registry (0x00-0x0F spec + 0x10-0xFF AxonOS)
│   ├── stim_guard.rs       # StimGuard integration (feature: stim-guard)
│   └── codec/
│       ├── mod.rs
│       ├── cbor.rs         # CBOR encoder (no_std compatible)
│       └── json.rs         # JSON encoder (feature: json, for relay boundary)
└── tests/
    └── vectors/
        └── consent-interop-vectors-v0.1.0.json   # ← Interop test vectors
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `cbor` | ✅ | CBOR codec for local IPC |
| `json` | ❌ | JSON codec for relay boundary (requires std) |
| `std` | ❌ | Standard library support |
| `stim-guard` | ❌ | StimGuard hardware integration for bidirectional BCI |

## Test Vectors

`tests/vectors/consent-interop-vectors-v0.1.0.json` contains **15 test cases** covering:

- All three frame types (withdraw, suspend, resume)
- Both scopes (peer, all)
- All spec-reserved reason codes (0x00–0x03)
- AxonOS implementation-specific codes (0x10–0x13)
- Edge cases: idempotent operations, unknown reason codes, timestamp precedence
- Gossip encoding (2-bit compact for BLE ATT MTU)
- BCI-specific: StimGuard lockout, emergency button bypass

**For Hongwei / MMP interop testing:**
Feed the `json` field of each test vector into the sym Node.js frame parser. If unknown frame types are silently ignored (per MMP Section 7 forward compatibility), the interop surface is correct.

## AxonOS Reason Code Registry

| Code | Name | Description |
|------|------|-------------|
| 0x00 | UNSPECIFIED | No reason given |
| 0x01 | USER_INITIATED | User or operator requested |
| 0x02 | SAFETY_VIOLATION | Safety constraint violated |
| 0x03 | HARDWARE_FAULT | Hardware fault detected |
| **0x10** | **STIMGUARD_LOCKOUT** | Cognitive Hypervisor triggered lockout |
| 0x11 | SESSION_ATTESTATION_FAILURE | Neural authentication mismatch |
| 0x12 | EMERGENCY_BUTTON | Physical emergency button |
| 0x13 | SWARM_FAULT_DETECTED | Byzantine node behaviour |

## Enforcement Path (Bidirectional BCI)

```
consent-withdraw received (or physical button pressed)
    ↓
nsc_withdraw_consent() — NSC gateway (NS → S transition)
    ↓
ConsentEngine.set_withdrawn() — state update in Secure World
    ↓
StimGuard.consent_withdrawn() — function call, same Secure World context
    ↓
Secure GPIO DAC gate close — single register write
    ↓
Total steps 3-4-5: <1µs, atomic in Secure World
```

## Conformance

This implementation targets **Safety-Critical Conformance** (Section 10.2) with all 13 conformance items.

## Licence

MIT — same as AxonOS core crates.

## Links

- **AxonOS**: [axonos.org](https://axonos.org) · [medium.com/@AxonOS](https://medium.com/@AxonOS) · [github.com/AxonOS-org](https://github.com/AxonOS-org)
- **MMP**: [sym.bot/spec/mmp](https://sym.bot/spec/mmp) · [github.com/sym-bot](https://github.com/sym-bot)
- **Contact**: axonosorg@gmail.com
