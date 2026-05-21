<div align="center">

# axonos-consent

### Protocol-level consent enforcement for AxonOS — a solo specification by Denis Yermakou.

[![Crate](https://img.shields.io/badge/crate-v0.3.0-orange)](./Cargo.toml)
[![Specification](https://img.shields.io/badge/spec-v0.3.0-blue)](./SPEC.md)
[![License: CC-BY-SA-4.0 (spec)](https://img.shields.io/badge/spec--license-CC--BY--SA--4.0-lightgrey.svg)](./LICENSE-CC-BY-SA)
[![License: Apache-2.0 OR MIT (code)](https://img.shields.io/badge/code--license-Apache--2.0%20OR%20MIT-blue.svg)](./LICENSE)
[![Verified: Kani BMC](https://img.shields.io/badge/verified-Kani%20BMC-success)](./kani/)

[Specification](./SPEC.md) · [Architecture](./docs/ARCHITECTURE.md) · [Security model](./docs/SECURITY-MODEL.md) · [Test vectors](./vectors/)

</div>

---

## What this repository is

This repository contains:

1. The **AxonOS Consent Specification v0.3.0** — a solo specification by Denis Yermakou, defining the kernel-level state machine that mediates user permission for `IntentObservation` flow in a conformant AxonOS deployment.
2. The **reference Rust implementation** — `#![no_std]` for ARMv8-M Cortex-M targets.
3. The **Kani Bounded Model Checking harnesses** that produce the L1 evidence backing every timing claim.
4. The **conformance test vectors** that any independent implementation must pass.

This is a **standalone subsystem of the AxonOS Project**. It has no external co-authors and no external coupling-protocol dependencies. The specification is downstream of the [AxonOS Standard](https://github.com/AxonOS-org/axonos-standard) §6.

## The consent state machine in one diagram

```
       ┌───────────┐
       │  Granted  │◄────────────┐
       └─────┬─────┘             │
             │                   │
   user pause│  user resume      │
             ▼                   │
       ┌───────────┐             │
       │ Suspended │─────────────┘
       └─────┬─────┘
             │
   user revoke (also from Granted)
             ▼
       ┌───────────┐
       │ Withdrawn │  (terminal — requires new manifest install)
       └───────────┘
```

`Withdrawn` is terminal. Only path back is a fresh manifest install through the trusted path. This non-reversibility is the central anti-coercion property.

## Performance envelope (reference hardware: STM32F407 @ 168 MHz)

| Property | Value | Evidence |
|:---|---:|:---:|
| Cycles per transition (upper bound) | **≤ 1648** | L1 (Kani-proven) |
| Wall-clock per transition (upper bound) | **≤ 9.8 µs** | L1 |
| End-to-end withdrawal → stream termination | **≤ 10 ms** | L1 composition |
| Median (measured, 18-h soak, 12 × 10⁶ events) | 1098 cycles ≈ 6.5 µs | L2 |
| 99.9th percentile (measured) | 1487 cycles ≈ 8.85 µs | L2 |
| Worst observed (measured) | 1503 cycles ≈ 8.95 µs | L2 |
| Source lines | 1,890 | — |
| Files | 18 | — |
| Allocations on critical path | 0 | static analysis |
| Kani harnesses | 4 | passing at v0.3.0 |

All measurements within the L1 bound; no Kani counterexamples at v0.3.0.

## Repository layout

```
axonos-consent/
├── SPEC.md                  ← The canonical specification (this is the source of truth)
├── README.md                ← This file
├── CHANGELOG.md             ← Version history; v0.3.0 is a clean restart
├── LICENSE                  ← Apache-2.0 OR MIT for code
├── LICENSE-APACHE           ← Apache-2.0 full text
├── LICENSE-MIT              ← MIT full text
├── LICENSE-CC-BY-SA         ← CC-BY-SA-4.0 full text for the specification
├── Cargo.toml               ← Crate manifest; pinned to MSRV 1.85
├── src/                     ← Reference Rust implementation
├── kani/                    ← Bounded-model-checking harnesses (L1 evidence)
├── tests/                   ← Unit + integration + property tests
├── benches/                 ← L2 measurement harnesses
├── examples/                ← How to use the crate
├── vectors/                 ← Conformance test vectors (wire-format)
├── docs/                    ← Architecture, security model, design rationale
└── .github/workflows/       ← CI: tests on 3 host OSes, no_std build, Kani, lint, security audit
```

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

let mut machine = ConsentMachine::new(manifest_id, trusted_path_pubkey);
assert_eq!(machine.state(), ConsentState::Granted);

let event: ConsentEvent = receive_from_trusted_path();
match machine.handle_event(event) {
    Ok(new_state) => { /* state updated; publish errors to suspended/withdrawn streams */ },
    Err(e)        => { /* refused: invalid signature, inadmissible transition, or wire-format error */ },
}
```

A worked example is in [`examples/basic_usage.rs`](./examples/basic_usage.rs).

## Verifying L1 claims

```sh
# Install Kani once
cargo install --locked kani-verifier
cargo kani setup

# Run all four harnesses
cargo kani --harness handle_withdraw_terminates
cargo kani --harness fsm_no_invalid_transitions
cargo kani --harness cbor_decoder_bounded
cargo kani --harness signature_verification_constant_time
```

Each harness prints `VERIFICATION SUCCESSFUL` on a passing run. A counterexample, if any, is reported with the input that violates the bound.

## Conformance against the specification

A separate implementation can run the conformance vectors:

```sh
cargo test --test conformance_vectors
```

If your implementation is in another language, the vectors are exported in canonical binary form at [`vectors/`](./vectors/) and can be replayed by any wire-format-aware driver.

## Versioning

| Version | Status | Notes |
|:---|:---:|:---|
| **v0.3.0** | **current** | Solo specification; clean restart; v1.0.0 of the spec text |
| v0.2.x | superseded | pre-restart drafts |
| v0.1.x | superseded | early drafts |

A v1.0.0 crate release will accompany the second independent implementation of the v0.3.0 specification. Until then, the crate remains `0.y.z` to reflect that the implementation is not yet locked.

The **specification text** is at v0.3.0 and is stable; the *crate* may iterate at the patch level (`v0.3.x`) for bug fixes and ergonomic improvements without modifying the specification.

## Authorship

This repository is authored solely by **Denis Yermakou**.

- Specification: [SPEC.md](./SPEC.md) — Denis Yermakou, AxonOS Project, Singapore.
- Reference implementation: same author, same project.
- No external co-authors. No external coupling-protocol dependencies.

Inquiries: [info@axonos.org](mailto:info@axonos.org). Security: [security@axonos.org](mailto:security@axonos.org).

## Licensing

- **Specification text** (`SPEC.md`, `docs/`, `README.md`): [CC-BY-SA-4.0](./LICENSE-CC-BY-SA).
- **Source code** (`src/`, `tests/`, `benches/`, `examples/`): [Apache-2.0 OR MIT](./LICENSE) at your option.
- **Test vectors** (`vectors/`): [CC0-1.0](./vectors/LICENSE) — interoperability vectors are dedicated to the public domain so any conformant implementation can use them freely.

---

<div align="center">

**axonos-consent · v0.3.0**

Singapore · Zurich · Berlin · Milano · San Mateo

</div>
