<div align="center">

# axonos-consent

**Consent for brain–computer interfaces, enforced by the kernel.**<br>
Authenticated with Ed25519. Replay-proof. Final the instant it is withdrawn.

<sub>The reference implementation of the <a href="./SPEC.md">AxonOS Consent Specification 0.6.0</a> · Rust · <code>no_std</code> · part of <a href="https://axonos.org">AxonOS</a></sub>

<br>

[![CI](https://img.shields.io/github/actions/workflow/status/AxonOS-org/axonos-consent/ci.yml?branch=main&style=flat-square&label=CI&labelColor=0d1117)](https://github.com/AxonOS-org/axonos-consent/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/AxonOS-org/axonos-consent?style=flat-square&label=release&labelColor=0d1117&color=1f8fae)](https://github.com/AxonOS-org/axonos-consent/releases)
[![Proofs](https://img.shields.io/badge/proofs-Kani%20%C2%B7%2010%20harnesses-0d7a5f?style=flat-square&labelColor=0d1117)](./src/proofs.rs)
[![Concurrency](https://img.shields.io/badge/concurrency-loom%20model--checked-0d7a5f?style=flat-square&labelColor=0d1117)](./src/gate.rs)
[![Conformance](https://img.shields.io/badge/conformance-20%20vectors%20%C2%B7%20CC0-1f8fae?style=flat-square&labelColor=0d1117)](./vectors/)

[![no_std](https://img.shields.io/badge/no__std-yes-475569?style=flat-square&labelColor=0d1117)](./src/lib.rs)
[![unsafe](https://img.shields.io/badge/unsafe-forbidden-475569?style=flat-square&labelColor=0d1117)](./src/lib.rs)
[![Ed25519](https://img.shields.io/badge/Ed25519-RFC%208032%20strict-475569?style=flat-square&labelColor=0d1117)](./SPEC.md#7-authentication)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-475569?style=flat-square&labelColor=0d1117)](./Cargo.toml)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-475569?style=flat-square&labelColor=0d1117)](./LICENSING.md)

[![AxonOS Standard](https://img.shields.io/badge/AxonOS-Standard%20v1.0.0-1f8fae?style=flat-square&labelColor=0d1117)](https://github.com/AxonOS-org/axonos-standard)
[![AxonOS Radar](https://img.shields.io/endpoint?url=https%3A%2F%2Faxonos-bci.github.io%2Faxonos-community-radar%2Fbadges%2FAxonOS-org%2Faxonos-consent.json&style=flat-square&labelColor=0d1117)](https://axonos-bci.github.io/axonos-community-radar/)
[![axonos.org](https://img.shields.io/badge/axonos.org-project-1f8fae?style=flat-square&labelColor=0d1117)](https://axonos.org)

**[Specification](./SPEC.md)** · **[Security model](./docs/SECURITY-MODEL.md)** · **[Architecture](./docs/ARCHITECTURE.md)** · **[Conformance vectors](./vectors/)** · **[Changelog](./CHANGELOG.md)** · **[Security policy](./SECURITY.md)**

</div>

> [!IMPORTANT]
> **0.9.0 is a security release.** Up to and including 0.8.0, the signature check was a 4-byte tag computed from the *public* trusted-path key, so anyone who knew that key could forge a withdrawal, a suspension or a resumption. 0.9.0 verifies a full Ed25519 signature on every frame and refuses every replay. Upgrade, and read [AXC-2026-001](./docs/advisories/AXC-2026-001.md).

---

## Why it exists

A brain–computer interface reads signals the person producing them cannot hide. Consent to that reading cannot be a setting an application chooses to honour; it has to be a fact the kernel enforces before any observation leaves it. `axonos-consent` is that enforcement point in [AxonOS](https://axonos.org): a three-state machine — `Granted`, `Suspended`, `Withdrawn` — driven only by signed decisions from the trusted path, and a publication gate that every observation must pass.

## Four guarantees

| | Guarantee | Established by |
|:--:|:--|:--|
| **1** | **Nothing changes without a signature.** A frame becomes a decision only after its Ed25519 signature verifies under the key of the party it names. Knowing the public key is worth nothing. | Kani proof, a test that flips each of the 768 bits of a frame, a forgery fuzzer |
| **2** | **Nothing is admitted twice.** Every frame carries a sequence number that must exceed the last one consumed from its signer, and the counter survives a power cycle. | Kani proof, tests |
| **3** | **Withdrawal is final.** `Withdrawn` is absorbing in the state machine and in the gate, and a corrupted state byte reads as `Withdrawn`, never as `Granted`. | Three Kani proofs |
| **4** | **Withdrawal is immediate.** Consent and the publication count share one atomic word; once a withdrawal is stored, no observation can be committed. | loom, every interleaving; a 200-round test on real threads |

## How a decision is admitted

```text
 frame — 96 bytes from the trusted path
   │
   ├─ 1  wire      exactly 96 bytes · magic AXC2 · canonical record      refused → 0x07
   ├─ 2  auth      strict Ed25519 under the key of the party it names    refused → 0x08
   ├─ 3  sequence  greater than the last consumed from that party        refused → 0x08
   ├─ 4  fsm       admissible from the current state                     refused → 0xFF
   │
   ▼
 gate — one AtomicU32: consent state + publication count
   ▲
   └─ try_publish() — the IPC producer commits every observation through it
```

Every layer has one job and one proof, and the order is normative ([SPEC §7.6](./SPEC.md#76-order-of-checks)): two implementations refuse the same frame for the same reason.

## Quick start

```toml
[dependencies]
axonos-consent = { git = "https://github.com/AxonOS-org/axonos-consent", tag = "v0.9.1" }
```

```rust
use axonos_consent::{ConsentMachine, Ed25519Strict};

// The kernel holds only the trusted path's public key.
let mut consent = ConsentMachine::new(manifest_id, trusted_path_public_key, Ed25519Strict)?;

// Trusted path → kernel: one signed 96-byte frame.
match consent.handle(&frame) {
    Ok(_state) => storage.write(consent.persisted()), // persist before acknowledging
    Err(refusal) => audit.record(refusal.to_abi_code()),
}

// IPC producer: write the slot, then commit it through the gate.
ring.write(consent.gate().published(), observation);
if let Err(suppressed) = consent.gate().try_publish() {
    sdk.deliver(suppressed.abi_code()); // 0x05 suspended · 0x06 withdrawn
}
```

Runnable versions: [`basic_usage`](./examples/basic_usage.rs), [`dual_control`](./examples/dual_control.rs) and [`publication_gate`](./examples/publication_gate.rs), which races a producer thread against a withdrawal.

| Feature | Default | What it does |
|:--|:--:|:--|
| `ed25519` | ✓ | The reference verifier, `Ed25519Strict`, on `ed25519-dalek` |
| `ed25519-fast` | | Precomputed tables: faster verification for tens of KiB of flash |
| `std` | | `std::error::Error` for `ConsentError` |

Without `ed25519`, bring your own [`SignatureVerifier`](./src/auth.rs) — a secure element, for instance. The crate then has no dependencies at all.

## The wire format

```text
 offset  size  field          
      0     4  magic          "AXC2" — protocol and version; the domain separator
      4     1  state          0x01 Granted · 0x02 Suspended · 0x03 Withdrawn
      5     1  flags          bit 0 terminal · bit 1 from-secure-world · bit 3 guardian
      6     2  manifest_id    u16 LE
      8     8  sequence       u64 LE · strictly increasing per signer
     16     8  timestamp_us   u64 LE · the signer's clock · informational only
     24     8  reserved       zero
     32    64  signature      Ed25519 (RFC 8032) over bytes 0..32
```

The terminal flag must agree with the state, the role is signed, and the signer's clock decides nothing. The full rules are in [SPEC §6](./SPEC.md#6-wire-format).

## Dual control

For clinical deployments a guardian holds a second key. Either party can stop the flow alone; only both can resume it, within a window measured on the kernel's clock. The role travels inside the signed record, so a frame cannot be relabelled, and a machine refuses to be built with one key for both parties.

```rust
let mut consent = DualControlMachine::new(manifest_id, patient_key, guardian_key, Ed25519Strict)?;
consent.propose(&frame, kernel_monotonic_us)?; // Applied(state) or PendingCoAuth(state)
```

## Evidence

| Property | Evidence | Reproduce |
|:--|:--|:--|
| Decoding is total and canonical | Kani · fuzz `frame_decode` | `cargo kani --no-default-features` |
| No transition without authentication | Kani · 768-bit flip test · fuzz `auth_forgery` | `cargo test` |
| No sequence admitted twice, across a power cycle | Kani · tests | `cargo test` |
| `Withdrawn` is absorbing; corruption fails closed | Kani ×3 · exhaustive unit test | `cargo kani --no-default-features` |
| No publication after a withdrawal returns | loom, three models · real-thread test | `RUSTFLAGS="--cfg loom" cargo test --release --lib loom` |
| One party can stop, only two can resume | Kani ×2 · ten tests | `cargo test --test dual_control` |
| Strict Ed25519, malleability refused | 20 conformance vectors, RFC 8032 keys | `cargo run --example gen_vectors --features std -- --check` |

Every row runs in [CI](./.github/workflows/ci.yml) on every change, inside the aggregate gate that every merge must pass.

**What is not claimed.** No cycle or wall-clock bound: earlier releases published a cycle figure for a path that no longer exists, and a new one will appear only with its derivation and an on-device measurement. The correctness of Ed25519 itself is taken from `ed25519-dalek`, not proven here. Verification handles only public data; signing and its side channels belong to the trusted path. Key provisioning, secure boot and authenticated storage are the integrator's, as [SPEC §8](./SPEC.md#8-storage-and-persistence) and [§11](./SPEC.md#11-threat-model) set out. The fuzz jobs run a 60-second campaign per target: a regression net, not a search.

## Conformance

The twenty vectors in [`vectors/`](./vectors/) are the specification made executable: a frame, the state and sequence it meets, and the exact outcome — new state, or the refusal and its ABI code. They are signed with the RFC 8032 test keys, dedicated to the public domain, and checked byte for byte against the generator on every change. An implementation in any language conforms by producing every documented outcome.

## Integrating it

- **Persist before acknowledging.** Write `persisted()` — the state and the last sequence from each signer — atomically, after each admitted frame. A lost sequence number re-opens every old frame to replay.
- **Sign with fresh sequences.** The trusted path keeps its own counter, strictly increasing and itself persisted.
- **Give dual control the kernel's clock.** `now_us` comes from the monotonic clock, never from a frame.
- **Size the ring as a power of two,** at most 2^29 slots; the gate's count wraps at 2^30.
- **Bringing your own verifier?** Implement strict RFC 8032 verification, and refuse small-order keys in `accepts_key`.

## Versioning

| | Version | Source of truth |
|:--|:--|:--|
| Crate | **0.9.1** | `Cargo.toml` |
| Specification | **0.6.0** | `SPEC.md`, `SPEC_VERSION` |
| Wire format | **2** (`AXC2`) | `src/wire.rs`, SPEC §6 |

[`tools/check.py`](./tools/check.py) fails the build if any of them disagree, or if a badge on this page stops telling the truth. 0.9.0 is breaking by necessity: wire v1 could not carry a signature.

## Repository

```text
src/            wire · auth · state · machine · gate · dual_control · proofs (Kani)
tests/          the boundary under attack · dual control · real threads · vectors
vectors/        twenty conformance vectors, CC0
fuzz/           frame_decode · fsm_sequence · auth_forgery
examples/       basic_usage · dual_control · publication_gate · gen_vectors
docs/           architecture · security model · design rationale · advisories
SPEC.md         the AxonOS Consent Specification, CC-BY-SA-4.0
```

## In the AxonOS stack

| Repository | Role |
|:--|:--|
| [axonos-standard](https://github.com/AxonOS-org/axonos-standard) | The canonical Standard; consent and the trusted path are its Sections 15 and 16 |
| [axonos-kernel](https://github.com/AxonOS-org/axonos-kernel) | The real-time kernel whose IPC producer commits through the gate |
| [axonos-sdk](https://github.com/AxonOS-org/axonos-sdk) | Delivers `0x05` and `0x06` to applications |
| [axonos-conformance](https://github.com/AxonOS-org/axonos-conformance) | Conformance vectors across the RFCs |
| [AxonOS Radar](https://axonos-bci.github.io/axonos-community-radar/) | A living map of open neurotech, scored from public evidence |

## Acknowledgements

Ed25519 comes from [`ed25519-dalek`](https://github.com/dalek-cryptography/curve25519-dalek) and `curve25519-dalek` by the dalek-cryptography contributors (BSD-3-Clause). The test keys are those of [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032) §7.1 (Josefsson and Liusvaara, IETF, 2017). The proofs run on the [Kani Rust Verifier](https://github.com/model-checking/kani), and the concurrency models on [loom](https://github.com/tokio-rs/loom) from the Tokio project. The defects fixed in 0.9.0 were found in an independent review of 0.8.0.

## Licence and citation

Code under [Apache-2.0](./LICENSE-APACHE) or [MIT](./LICENSE-MIT), at your option. The specification text under [CC-BY-SA-4.0](./LICENSE-CC-BY-SA). The conformance vectors dedicated to the public domain under [CC0-1.0](./vectors/LICENSE). Why the licence files are arranged as they are: [LICENSING.md](./LICENSING.md). To cite this work, use [CITATION.cff](./CITATION.cff) — GitHub's *Cite this repository* reads it.

---

<div align="center">

**The AxonOS Project** · [axonos.org](https://axonos.org) · [connect@axonos.org](mailto:connect@axonos.org) · [security@axonos.org](mailto:security@axonos.org)<br>
[github.com/AxonOS-org](https://github.com/AxonOS-org) · [AxonOS Radar](https://axonos-bci.github.io/axonos-community-radar/)

<sub>© 2026 Denis Yermakou · `axonos-consent`</sub>

</div>
