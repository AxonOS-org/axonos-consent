# axonos-consent

**AxonOS-native deterministic neural consent runtime for `no_std` BCI systems.**

`axonos-consent` defines the AxonOS Consent State Model: runtime consent state,
terminal withdrawal semantics, capability-gated delivery, bounded wire handling,
and observation-gate integration for neural software.

Consent is not a UI checkbox.

Consent is runtime safety state enforced below the application layer.

## Status

| Field | Value |
|---|---|
| Crate | `axonos-consent` |
| Version | `0.3.0` |
| Status | Draft reference implementation |
| Runtime target | `no_std` |
| Allocation policy | no heap on critical path |
| Unsafe policy | `#![forbid(unsafe_code)]` |
| Clinical claim | none |
| Regulatory claim | none |
| L3 timing claim | none |
| External protocol compatibility claim | none |

This repository does not claim clinical deployment readiness, regulatory
approval, final AxonOS conformance, or compatibility with external consent
protocols.

## What this repository is

This repository contains:

1. The AxonOS Consent Specification v0.3.0.
2. A reference Rust implementation for embedded and hosted testing.
3. Bounded model-checking harnesses for L1 evidence.
4. Conformance test vectors for independent implementations.
5. A draft observation-gate interface for delivery control.

## What this repository is not

This repository is not:

- a medical-device approval package;
- a clinical protocol;
- a regulatory submission;
- a legal consent document;
- a patient-use authorization;
- a claim of L3 hardware timing validation;
- a compatibility implementation of an external consent protocol.

## AxonOS Standard mapping

| Standard artifact | Relevance |
|---|---|
| AOS-0004 Neural Permissions | consent interacts with neural access authority |
| AOS-0005 Consent Semantics | defines grant, suspension, withdrawal, expiry, and fault behavior |
| AOS-0009 Security and Privacy Threat Model | treats consent bypass as security-relevant |
| AOS-0012 Hardware Validation Protocol | governs future L3 timing and interlock traces |

## Consent state machine

The draft state model is intentionally small:

```text
Granted
Suspended
Withdrawn
```

`Withdrawn` is terminal for the current session.

A same-session attempt to resume delivery after withdrawal must fail closed.

## Performance and evidence posture

The repository may contain analytical and development-fixture measurements, but
public timing values must remain evidence-scoped.

| Claim family | Current status |
|---|---|
| state-machine behavior | L1: tests / source review |
| bounded parser behavior | L1: source review / tests |
| no unsafe code | L1: compile-time policy |
| no heap on critical path | L1: source review |
| timing values | analytical or fixture-scoped unless trace-linked |
| hardware-gate response | pending L3 under AOS-0012 |

No L3 hardware timing claim is made by this repository until external GPIO,
logic-analyzer, oscilloscope, or equivalent traces are published with metadata.

## Quick start

Add to your `Cargo.toml`:

```toml
[dependencies]
axonos-consent = "0.3"
```

Use:

```rust
#![no_std]

use axonos_consent::{ConsentMachine, ConsentState, ConsentEvent};

let manifest_id: u16 = 1;
let trusted_path_pubkey = [0u8; 32];

let mut machine = ConsentMachine::new(manifest_id, trusted_path_pubkey);

assert_eq!(machine.state(), ConsentState::Granted);

let event: ConsentEvent = receive_from_trusted_path();

match machine.handle_event(event) {
    Ok(new_state) => {
        // State updated; publish errors to suspended/withdrawn streams.
    }
    Err(e) => {
        // Refused: invalid signature, inadmissible transition, or wire-format error.
    }
}
```

A worked example is in `examples/basic_usage.rs`.

## Repository layout

```text
axonos-consent/
├── SPEC.md
├── README.md
├── CHANGELOG.md
├── Cargo.toml
├── LICENSE
├── LICENSE-APACHE
├── LICENSE-MIT
├── LICENSE-CC-BY-SA
├── src/
│   ├── lib.rs
│   ├── crypto.rs
│   ├── error.rs
│   ├── interlock.rs
│   ├── state.rs
│   └── wire.rs
├── kani/
├── tests/
├── benches/
├── examples/
├── vectors/
├── docs/
├── tools/
└── .github/workflows/
```

## CI contract

The repository CI verifies:

- public-surface formatting;
- line counts;
- Cargo manifest readability;
- README scope;
- Rustdoc surface;
- clean contact policy;
- no collapsed workflow;
- no stale external-protocol narrative in primary public files;
- no clinical or regulatory overclaim;
- security policy;
- license files;
- source tree presence.

## Contact

General: connect@axonos.org

Security: security@axonos.org
