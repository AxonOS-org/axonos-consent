<div align="center">

# axonos-consent

### Protocol-level consent enforcement for AxonOS.

#### A kernel-level finite-state machine with formally bounded withdrawal latency.

[![CI](https://github.com/AxonOS-org/axonos-consent/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/AxonOS-org/axonos-consent/actions/workflows/ci.yml)
[![Crate](https://img.shields.io/badge/Crate-v0.5.0-0a4a8f?style=flat-square)](https://github.com/AxonOS-org/axonos-consent/releases/tag/v0.5.0)
[![Spec](https://img.shields.io/badge/Spec-v0.5.0-0a4a8f?style=flat-square)](./SPEC.md)
[![Standard](https://img.shields.io/badge/Standard-v1.0.0-0a4a8f?style=flat-square)](https://github.com/AxonOS-org/axonos-standard)
[![Rust](https://img.shields.io/badge/Rust-no__std-CE422B?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)

[![Verified](https://img.shields.io/badge/Verified-Kani%20BMC%20%C3%975-0d7a5f?style=flat-square)](./kani/)
[![Unsafe](https://img.shields.io/badge/Unsafe-forbidden-0d7a5f?style=flat-square)](./src/lib.rs)
[![Critical path](https://img.shields.io/badge/Critical%20path-0%20alloc-0d7a5f?style=flat-square)](./SPEC.md#4-timing-bounds)
[![WCRT](https://img.shields.io/badge/WCRT-%E2%89%A4%201648%20cycles-0d7a5f?style=flat-square)](./SPEC.md#4-timing-bounds)

[![License (code)](https://img.shields.io/badge/License%20code-Apache--2.0%20OR%20MIT-475569?style=flat-square)](./LICENSE)
[![License (spec)](https://img.shields.io/badge/License%20spec-CC--BY--SA--4.0-475569?style=flat-square)](./LICENSE-CC-BY-SA)
[![MSRV](https://img.shields.io/badge/MSRV-1.75-475569?style=flat-square)](./Cargo.toml)

---

[**Specification**](./SPEC.md) · [Architecture](./docs/ARCHITECTURE.md) · [Security model](./docs/SECURITY-MODEL.md) · [Design rationale](./docs/DESIGN-RATIONALE.md) · [Test vectors](./vectors/) · [Changelog](./CHANGELOG.md)

</div>

---

## What this repository is

1. The **[AxonOS Consent Specification](./SPEC.md)** — a solo specification by Denis Yermakou, defining the kernel-level state machine that mediates user permission for `IntentObservation` flow in a conformant AxonOS deployment.
2. The **reference Rust implementation** — `#![no_std]`, `#![forbid(unsafe_code)]`, targeting ARMv8-M Cortex-M.
3. The **Kani Bounded Model Checking harnesses** that produce the L1 evidence backing every timing claim.
4. The **conformance test vectors** that any independent implementation must pass, dedicated to the public domain under CC0-1.0.

This is a **standalone subsystem of the AxonOS Project**. No external co-authors. No external coupling-protocol dependencies. The specification is downstream of the [AxonOS Standard](https://github.com/AxonOS-org/axonos-standard) §6.

---

## The consent state machine

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

`Withdrawn` is terminal. The only path back is a fresh manifest install through the trusted path. **This non-reversibility is the central anti-coercion property.**

---

## Performance envelope (reference hardware: STM32F407 @ 168 MHz)

| Property | Value | Evidence level |
|:---|---:|:---:|
| Cycles per transition (upper bound) | **≤ 1648** | L1 (Kani-proven) |
| Wall-clock per transition (upper bound) | **≤ 9.8 µs** | L1 |
| End-to-end withdrawal → stream termination | **≤ 10 ms** | L1 composition |
| Median (measured, 18-h soak, 12 × 10⁶ events) | 1098 cycles · ≈ 6.5 µs | L2 |
| 99.9th percentile (measured) | 1487 cycles · ≈ 8.85 µs | L2 |
| Worst observed (measured) | 1503 cycles · ≈ 8.95 µs | L2 |
| Soak duration with zero unsafe states | 18 h / 12 × 10⁶ events | L2 |
| Critical-path allocations | 0 | static analysis |
| Source lines (`src/`) | 594 | — |
| Unsafe blocks | 0 | `#![forbid(unsafe_code)]` |
| Kani harnesses | 5 | passing at v0.5.0 |

All measurements remain within the L1 bound. No Kani counterexamples are known for the current verification surface.

---

## Continuous integration

Every push and pull-request runs the full CI matrix in [.github/workflows/ci.yml](./.github/workflows/ci.yml). The eight CI jobs:

| Job | What it checks | Blocking |
|:---|:---|:---:|
| `Format (rustfmt)` | `cargo fmt --all --check` — source is `cargo fmt`-clean | ✅ |
| `Lint (clippy)` | `cargo clippy --all-features --all-targets` — no lint errors | ✅ |
| `Test (ubuntu, stable)` | `cargo test` with both `--all-features` and `--no-default-features` | ✅ |
| `Build no_std (Cortex-M4F)` | `cargo build --target thumbv7em-none-eabihf --no-default-features --release` | ✅ |
| `Documentation (rustdoc)` | `cargo doc --no-deps` with `RUSTDOCFLAGS=-D warnings` (no broken intra-doc links) | ✅ |
| `License files & SPDX` | All five LICENSE files present with correct SPDX identifiers | ✅ |
| `Fuzz (build + 60s smoke)` | Builds the three `cargo-fuzz` targets and smoke-runs each for 60 s on nightly | ✅ |
| `CI` (aggregate) | Green check iff every job above passed | ✅ |

A red X on any job blocks the merge. The aggregate `CI` job is what the branch-protection rule watches.

---

## Repository layout

```
axonos-consent/
├── SPEC.md                  ← canonical specification (this is the source of truth)
├── README.md                ← this file
├── CHANGELOG.md             ← version history; v0.5.0 is the current release
├── Cargo.toml               ← crate manifest; MSRV 1.75
├── LICENSE                  ← Apache-2.0 OR MIT dispatcher for code
├── LICENSE-APACHE           ← Apache-2.0 full text
├── LICENSE-MIT              ← MIT full text
├── LICENSE-CC-BY-SA         ← CC-BY-SA-4.0 full text for the specification
├── rustfmt.toml             ← formatting configuration
├── rust-toolchain.toml      ← pins stable + rustfmt + clippy + thumbv7em
│
├── src/                     ← reference Rust implementation (#![no_std])
│   ├── lib.rs               ← crate root, exports, doctest
│   ├── state.rs             ← consent FSM with AtomicU8
│   ├── wire.rs              ← 16-byte little-endian wire format
│   ├── crypto.rs            ← constant-time signature verification
│   ├── error.rs             ← typed error taxonomy
│   ├── interlock.rs         ← ObservationGate trait for kernel IPC integration
│   └── dual_control.rs      ← multi-party (guardian) co-authorisation (v0.5.0)
│
├── kani/                    ← Bounded-model-checking harnesses (L1 evidence)
│   ├── handle_withdraw_terminates.rs
│   ├── fsm_no_invalid_transitions.rs
│   ├── cbor_decoder_bounded.rs
│   ├── signature_verification_constant_time.rs
│   └── co_authorisation_requires_two_parties.rs
│
├── tests/                   ← unit + integration + property tests
│   ├── integration.rs       ← full FSM lifecycle
│   └── wire_format.rs       ← wire-format roundtrip + refusal cases
│
├── benches/                 ← L2 measurement harnesses
│   └── withdrawal_latency.rs
│
├── examples/                ← worked usage examples
│   ├── basic_usage.rs       ← (requires the `std` feature)
│   └── dual_control.rs      ← guardian co-authorisation walkthrough
│
├── vectors/                 ← conformance test vectors (CC0-1.0; public domain)
│   ├── README.md
│   └── LICENSE
│
├── fuzz/                    ← coverage-guided fuzz suite (cargo-fuzz; L2 evidence)
│   ├── fuzz_targets/        ← wire_decode, roundtrip, fsm_sequence
│   ├── corpus/              ← committed seed corpus
│   └── README.md            ← how to build, run, and triage
│
├── docs/                    ← informative companion documents
│   ├── ARCHITECTURE.md
│   ├── SECURITY-MODEL.md
│   ├── DESIGN-RATIONALE.md
│   └── citation.bib
│
└── .github/workflows/
    └── ci.yml               ← 8-job CI: fmt, clippy, test, no_std build, docs, license, fuzz, aggregate
```

---

## Quick start

Add to your `Cargo.toml`:

```toml
[dependencies]
axonos-consent = "0.4"
```

Use:

```rust
use axonos_consent::{ConsentMachine, ConsentState};

let manifest_id: u16 = 1;
let trusted_path_pubkey = [0u8; 32];  // Ed25519 public key
let machine = ConsentMachine::new(manifest_id, trusted_path_pubkey);
assert_eq!(machine.state(), ConsentState::Granted);
```

A worked example covering the full FSM lifecycle is in [`examples/basic_usage.rs`](./examples/basic_usage.rs).

---

## Verifying the L1 claims

```sh
# Install Kani once
cargo install --locked kani-verifier
cargo kani setup

# Run all five harnesses
cargo kani --harness handle_withdraw_terminates
cargo kani --harness fsm_no_invalid_transitions
cargo kani --harness cbor_decoder_bounded
cargo kani --harness signature_verification_constant_time
```

Each harness prints `VERIFICATION SUCCESSFUL` on a passing run. A counterexample, if any, is reported with the input that violates the bound.

---

## Fuzz and differential testing

Alongside the L1 Kani harnesses, the reference implementation carries a
coverage-guided fuzz suite in [`fuzz/`](./fuzz/), built on `cargo-fuzz` /
libFuzzer. Three targets search the unbounded input space for a
specification or implementation defect:

| Target | Surface | Property |
|:---|:---|:---|
| `wire_decode` | §6 wire-format decoder | totality — never panics on any byte buffer |
| `roundtrip` | §6 encode/decode | canonical encoding — no two buffers denote one event |
| `fsm_sequence` | §2–§3 state machine | FSM invariants under arbitrary signed-event streams |

```sh
cargo install cargo-fuzz --locked
cargo +nightly fuzz run wire_decode      # or roundtrip, fsm_sequence
```

The Kani harnesses are L1 evidence (exhaustive proof over a bounded space);
fuzzing is L2-class evidence (a large, coverage-guided sample of the unbounded
space). CI builds all three targets and smoke-runs each for 60 s on every
change. See [`fuzz/README.md`](./fuzz/README.md) and SPEC §10.3.

---

## Conformance against the specification

Independent implementations can run the conformance vectors:

```sh
cargo test --test conformance_vectors
```

For implementations in languages other than Rust, the vectors are exported in canonical binary form at [`vectors/`](./vectors/) under CC0-1.0; replay with any wire-format-aware driver.

---

## Versioning

| Version | Status | Notes |
|:---|:---:|:---|
| **v0.4.0** | **current** | Verification release — adds the `cargo-fuzz` suite and SPEC §10.3; protocol byte-identical to v0.3.0 |
| v0.3.0 | previous | Solo specification; clean restart; v1.0.0-equivalent of the spec text |
| v0.2.x | superseded | pre-restart drafts |
| v0.1.x | superseded | early drafts |

A v1.0.0 crate release will accompany the second independent implementation. Until then, the crate remains `0.y.z` to reflect that the implementation surface is not yet locked.

The single-party **specification protocol** is stable as of v0.3.0 and unchanged through v0.5.0. v0.4.0 recorded added validation evidence (SPEC §10.3); v0.5.0 adds the optional multi-party (guardian) co-authorisation profile (SPEC §12) without altering the single-party baseline. An implementation conformant with the v0.4.0 protocol is conformant with the v0.5.0 baseline profile without modification.

---

## Multi-party (guardian) co-authorisation (v0.5.0)

For clinical deployments — the ALS rehabilitation pilot in the canonical
Standard's roadmap is the motivating case — a guardian can co-authorise consent
changes together with the patient. This is the optional [`dual_control`](./src/dual_control.rs)
layer, specified normatively in [SPEC §12](./SPEC.md#12-multi-party-guardian-co-authorisation).

It follows the **safe-direction principle**:

- **Either party** may reduce neural-data exposure (`Suspended`, `Withdrawn`)
  **unilaterally**. The flow can always be stopped by one signature.
- **Resuming** the flow (`Suspended → Granted`) requires **both** parties to
  authorise the same transition within a bounded window. No sequence of
  signatures from one party can resume it — a property proven by the Kani
  harness `co_authorisation_requires_two_parties`.

The single-party `ConsentMachine` is unchanged; multi-party is opt-in by using
`DualControlMachine` instead. See [`examples/dual_control.rs`](./examples/dual_control.rs).

---

## Position in the AxonOS stack

| Layer | Repository | Role |
|---|---|---|
| Canonical standard | [`axonos-standard`](https://github.com/AxonOS-org/axonos-standard) | Architecture manual, conformance criteria, validation taxonomy |
| Engineering RFCs | [`axonos-rfcs`](https://github.com/AxonOS-org/axonos-rfcs) | Numbered design proposals; normative once finalised |
| Kernel substrate | [`axonos-kernel`](https://github.com/AxonOS-org/axonos-kernel) | EDF scheduling, SPSC IPC, capability gate, monotonic time |
| Application boundary | [`axonos-sdk`](https://github.com/AxonOS-org/axonos-sdk) | Typed intents, manifests, ABI-compatible integration |
| **Consent layer** | **`axonos-consent`** | Deterministic consent FSM + optional multi-party co-authorisation (this repository) |
| Mesh coordination | [`axonos-swarm`](https://github.com/AxonOS-org/axonos-swarm) | Distributed timing, co-availability, peer health monitoring |
| Acquisition gateway | [`axon-bci-gateway`](https://github.com/AxonOS-org/axon-bci-gateway) | OpenBCI GUI integration fork for EEG input |

---

## Authorship

This repository is authored solely by **Denis Yermakou** — AxonOS Project, Singapore.

- Specification text: [SPEC.md](./SPEC.md) — Denis Yermakou.
- Reference implementation: same author, same project.
- No external co-authors. No external coupling-protocol dependencies.

Inquiries: [connect@axonos.org](mailto:connect@axonos.org) · Security: [security@axonos.org](mailto:security@axonos.org).

---

## Licensing

| Surface | License |
|:---|:---|
| Source code (`src/`, `tests/`, `benches/`, `examples/`) | [**Apache-2.0 OR MIT**](./LICENSE) at your option |
| Specification text (`SPEC.md`, `docs/`, `README.md`) | [**CC-BY-SA-4.0**](./LICENSE-CC-BY-SA) |
| Conformance test vectors (`vectors/`) | [**CC0-1.0**](./vectors/LICENSE) — public domain dedication |

The test vectors are CC0 specifically so any independent implementation — in any language, under any license, commercial or otherwise — can use them without compatibility concerns.

---

<div align="center">

**axonos-consent · v0.5.0 · multi-party co-authorisation**

Singapore · Zurich · Berlin · Milano · San Mateo

</div>
