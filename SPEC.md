# AxonOS Consent Specification

**Version 0.5.0** · 2026-05-28 · Normative

**Author:** Denis Yermakou
**Project:** AxonOS
**Domicile:** Singapore
**License:** [CC-BY-SA-4.0](./LICENSE-CC-BY-SA) (specification text) · [Apache-2.0 OR MIT](./LICENSE) (reference code)

---

## Preface

This document is the canonical specification of the **AxonOS Consent** subsystem — the kernel-level state machine that mediates user permission for *IntentObservation* flow in any conformant AxonOS deployment. It is a self-contained subsystem of the AxonOS Project, authored solely by Denis Yermakou, with no external collaboration claims.

The specification is downstream of [the AxonOS Standard](https://github.com/AxonOS-org/axonos-standard) §6. Where this document elaborates the consent state machine, it does so without weakening the bounds set by the Standard; in the event of disagreement between this document and the Standard, the Standard wins.

This specification supersedes all prior drafts of consent semantics circulated in earlier internal documents, article series, or pre-public manuscripts. References in those documents to external coupling protocols, mesh extensions, or third-party collaborations are **not** part of this specification and are not normative for any conformant AxonOS Consent implementation.

The reference implementation is the `axonos-consent` crate at the version tagged in [`Cargo.toml`](./Cargo.toml).

Version 0.4.0 is editorially identical to v0.3.0 in protocol terms. The three-state machine, the five admissible transitions, the wire format, the timing bounds, and the cryptographic requirements are byte-for-byte unchanged. v0.4.0 differs from v0.3.0 only by the addition of the informative §10.3, which records the fuzz and differential-testing evidence for the reference implementation. An implementation conformant with v0.3.0 is conformant with v0.4.0 without modification.

Version 0.5.0 is a strict superset of v0.4.0. The single-party three-state machine, the five admissible transitions, the wire format, the timing bounds, and the cryptographic requirements are byte-for-byte unchanged. v0.5.0 adds the new, **optional** §12, which specifies multi-party (guardian) co-authorisation for clinical deployments, and promotes wire flag bit 3 (`FLAG_GUARDIAN`) from reserved to defined. An implementation conformant with v0.4.0 remains conformant with v0.5.0 without modification; multi-party support is an optional conformance profile.

---

## Document conventions

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this document are to be interpreted as described in RFC 2119 and RFC 8174.

Byte-level wire definitions are normative; the accompanying Rust types are informative.

---

## Contents

1. [Scope](#1-scope)
2. [The consent state machine](#2-the-consent-state-machine)
3. [Admissible transitions](#3-admissible-transitions)
4. [Timing bounds](#4-timing-bounds)
5. [Trusted path](#5-trusted-path)
6. [Wire format](#6-wire-format)
7. [Cryptographic verification](#7-cryptographic-verification)
8. [Storage and persistence](#8-storage-and-persistence)
9. [Kernel interlock](#9-kernel-interlock)
10. [Conformance](#10-conformance)
11. [Threat model](#11-threat-model)
12. [Multi-party (guardian) co-authorisation](#12-multi-party-guardian-co-authorisation)
13. [References](#13-references)

---

## 1. Scope

### 1.1 In scope

This specification defines:

- The three states of the consent finite-state machine (`Granted`, `Suspended`, `Withdrawn`).
- The admissible transitions between those states.
- The timing bounds each transition **MUST** satisfy.
- The wire format of consent events at the kernel/SDK boundary.
- The cryptographic verification each consent event **MUST** undergo before admission.
- The interlock by which a withdrawn consent terminates all open *IntentObservation* streams.
- The storage and persistence requirements across device power cycles.
- The conformance criteria for an implementation of this specification.

### 1.2 Out of scope

This specification does **not** define:

- The user-interface affordance through which the user signals a consent state change. That is the responsibility of the device manufacturer's UI layer; this specification only defines what arrives at the trusted-path input.
- The hardware design of the trusted path itself.
- The application layer's behaviour after receipt of a consent-related error code from the SDK. Applications **SHOULD** display a clear notification and stop attempting further observation requests; the specifics are application-domain decisions.
- Multi-party consent in its *single-party* baseline: the baseline machine of §2 is single-signature. Optional multi-party (guardian) co-authorisation is specified normatively in [§12](#12-multi-party-guardian-co-authorisation).
- Any inter-device coupling protocol. This specification is for a single-device AxonOS deployment; multi-device deployments use the [Standard's swarm coordination subsystem](https://github.com/AxonOS-org/axonos-standard/blob/main/architecture/swarm-coordination.md) layered above this one.

---

## 2. The consent state machine

### 2.1 The three states

A conformant implementation **MUST** model consent as a finite-state machine with exactly three states:

| State | Discriminant | Effect on observation streams |
|:---|:---:|:---|
| `Granted` | `0x01` | Observations flow normally. |
| `Suspended` | `0x02` | Observations do not flow; consumers receive a typed backpressure error. |
| `Withdrawn` | `0x03` | All streams terminated; manifest invalidated. |

Discriminant `0x00` is **reserved** and **MUST NOT** be emitted by any conformant implementation. Discriminants `0x04` through `0xFF` are **reserved**.

A consent state is associated with a specific *manifest installation*. The state is per-manifest, not per-device — different applications installed on the same device may be in different consent states simultaneously.

### 2.2 Initial state

A freshly installed manifest **MUST** begin in state `Granted`. There is no separate "pending" state; the act of installing a signed manifest through the trusted path constitutes initial consent.

A device factory reset **MUST** clear all manifest installations; there is no state to migrate.

### 2.3 State representation

Within the kernel, the per-manifest consent state **MUST** be stored as a single byte in a memory region with the following properties:

- Written only by the consent state machine's transition functions.
- Read by the kernel IPC publication path and by the optional Cognitive Hypervisor interlock.
- Not writable from any application-layer context.

The reference implementation places this byte in a `core::sync::atomic::AtomicU8` accessed with `Ordering::SeqCst` on writes and `Ordering::Acquire` on reads. Alternative implementations **MAY** use platform-specific synchronisation, provided the visibility guarantees of the seqlock pattern in [STANDARD §4.4](https://github.com/AxonOS-org/axonos-standard/blob/main/STANDARD.md#section-4) are preserved.

---

## 3. Admissible transitions

### 3.1 The transition graph

Exactly the following transitions are admissible:

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

The five admissible transitions are:

| From | To | Trigger |
|:---|:---|:---|
| `Granted` | `Suspended` | User pause from trusted path |
| `Suspended` | `Granted` | User resume from trusted path |
| `Granted` | `Withdrawn` | User revoke from trusted path |
| `Suspended` | `Withdrawn` | User revoke from trusted path |
| any | identity | Idempotent re-application of current state |

### 3.2 Inadmissible transitions

The following transitions are **NOT** admissible. An implementation receiving a request for any of these **MUST** ignore the request and emit a typed error:

- `Withdrawn → Granted`
- `Withdrawn → Suspended`
- Any transition initiated from a source other than the trusted path (§5).

### 3.3 Non-reversibility of `Withdrawn`

The `Withdrawn` state is **terminal**. The only path to receiving observations again is for the user to install a fresh manifest through the trusted path; a fresh manifest begins in `Granted` (§2.2) but is a new installation, not a resumption.

This non-reversibility is the central anti-coercion property of the consent system: if `Withdrawn → Granted` were admissible, an application or a privileged operator could pressure the kernel to silently move from `Withdrawn` back to `Granted`, defeating the user's revocation. The Standard treats this property as inviolable; it cannot be relaxed within the v1.x major version line.

### 3.4 Idempotency

Re-applying the current state through the trusted path **MUST** succeed without modifying state, **MUST NOT** emit an error, and **MUST** complete within the same timing bound as a non-trivial transition (§4). This permits the trusted path to recover from message loss without producing spurious state changes.

---

## 4. Timing bounds

### 4.1 The cycle bound

The state-machine transition function (the function in the reference implementation called `handle_event()`) **MUST** complete in **≤ 1648 CPU cycles** on the reference hardware (ARM Cortex-M4F at 168 MHz, equivalent to **≤ 9.8 µs**). The bound applies to any admissible input, including non-trivial transitions and idempotent re-applications.

**Evidence, stated precisely.** Two distinct claims live in this section and
they carry different evidence:

1. **The cycle figure (≤ 1648)** is an *analytical* bound, derived by
   instruction counting against the ISA timing reference. The derivation
   artefact is pending publication. It is **not** a Kani output. Kani is a
   bounded model checker over Rust MIR and does not compute Cortex-M cycle
   counts; a harness cannot produce a wall-clock or cycle bound.
2. **Termination and target-state correctness** is **L1** per the
   [AxonOS Standard validation taxonomy](https://github.com/AxonOS-org/axonos-standard/blob/main/VALIDATION.md),
   backed by the Kani harness `handle_withdraw_terminates`. That harness
   proves `handle_event()` terminates under bounded unwinding and yields
   `Withdrawn` on a terminal Withdraw frame.

At this revision the harness exercises the transition from `Granted` only;
extending it to the `Suspended` and `Withdrawn` starting states is a known
open item. Until the cycle-bound derivation is published and the harness
covers all three states, §4.1 must not be cited as a proven timing bound.

### 4.2 The wall-clock bound

A `* → Withdrawn` transition **MUST** terminate all open observation streams for the affected manifest within **≤ 10 ms wall-clock time** from receipt of the trusted-path withdrawal event.

This bound is composed of three sub-bounds:

| Component | Sub-bound | Evidence |
|:---|:---:|:---:|
| State-machine transition itself (§4.1) | ≤ 9.8 µs | analytical |
| Kernel IPC ring-buffer producer-side termination | ≤ 1 scheduler tick ≈ 4 ms | L1 |
| SDK observation iterator returning `StreamTerminated` | ≤ 1 SDK poll period ≈ 4 ms | L2 |
| **Sum** | ≤ 10 ms | composed |

### 4.3 Measured performance (reference hardware)

The reference implementation at v0.3.0 measures, on the reference hardware over an 18-hour soak with 12 × 10⁶ withdrawal events:

| Statistic | Value |
|:---|---:|
| Median withdrawal cycles | 1098 (≈ 6.5 µs) |
| 99.9th percentile cycles | 1487 (≈ 8.85 µs) |
| Worst observed cycles | 1503 (≈ 8.95 µs) |
| Analytical upper bound (§4.1) | 1648 (9.81 µs) |

All measurements fall within the analytical bound of §4.1. Kani produced no counterexample at the published correctness harness.

### 4.4 Bounds on adverse machine state

The analytical bound of §4.1 is intended to hold under the following adverse conditions:

- Cache cold-start (instruction and data caches both invalidated).
- Branch-predictor misses on the transition's control flow.
- DMA contention on the SRAM bus from the simultaneous IPC producer.
- Maximum admissible interrupt rate (set by [STANDARD §4.2](https://github.com/AxonOS-org/axonos-standard/blob/main/STANDARD.md#section-4)).

The Kani harness models all four. Implementations **SHOULD** test all four with measured benchmarks before claiming v0.3.0 conformance.

---

## 5. Trusted path

### 5.1 Definition

The **trusted path** is the input channel through which consent state transitions are signalled. A consent event arriving from any other source **MUST** be refused.

### 5.2 Acceptable trusted-path implementations

A conformant implementation **MUST** use one of the following as its trusted path:

- A **physical hardware button** wired directly to the kernel's input interrupt line, with debounce circuitry that emits at most one event per actuation regardless of bounce or noise.
- A **Secure-World UI partition** on an ARM TrustZone-M device (Cortex-M33), with the trusted-path event delivered to the Normal World only by a Secure Monitor Call from the Secure-World UI.
- An equivalent input channel that the application layer **provably cannot synthesise**.

### 5.3 Application-layer events are refused

A consent event whose origin is identified as an application-layer source (any source outside the trusted-path enumeration in §5.2) **MUST** be refused with a typed error. The implementation **MUST NOT** "convert" application-layer requests into trusted-path events under any circumstances.

This refusal includes:

- Network messages claiming to carry a consent transition.
- IPC messages from the application core.
- File-system writes to a configuration file.
- Environment-variable changes.

### 5.4 Audit

Every trusted-path event accepted by the consent state machine **SHOULD** be recorded in a tamper-evident audit log accessible only to the device operator (not the application). The audit log entry **MUST** include: the timestamp (from the kernel monotonic clock per [STANDARD §4.5](https://github.com/AxonOS-org/axonos-standard/blob/main/STANDARD.md#section-4)), the manifest ID affected, the from-state and to-state, and a cryptographic hash of the input event.

---

## 6. Wire format

### 6.1 The consent event record

A consent event crosses the trusted-path / kernel boundary as a **16-byte little-endian record**:

```
┌─ Offset ─┬─ Size ─┬─ Field ────────────┬─ Type ─────────────────┐
│   0      │   1    │ state              │ u8 (discriminant §2.1) │
│   1      │   1    │ flags              │ u8 (bitfield §6.2)     │
│   2      │   2    │ manifest_id        │ u16 (per-device)       │
│   4      │   8    │ timestamp_us       │ u64 (kernel monotonic) │
│  12      │   4    │ sig_truncated      │ u32 (Ed25519 tag §7.2) │
└──────────┴────────┴────────────────────┴────────────────────────┘
```

The encoding is canonical-CBOR-compatible at the data-model level (CBOR major type 4 array of five integer-typed members). Implementations **MAY** carry the record as raw 16 bytes when no CBOR encoder is available in the trusted-path partition.

### 6.2 Flags byte

| Bit | Meaning |
|:---:|:---|
| 0 | `terminal` — set to 1 if the encoded state is `Withdrawn` |
| 1 | `from-secure-world` — set to 1 if the event originated from a TrustZone-M Secure-World UI |
| 2 | `replay-tolerant` — set to 1 if the event is acceptable as an idempotent re-application |
| 3 | reserved (MUST be 0) |
| 4 | reserved (MUST be 0) |
| 5 | reserved (MUST be 0) |
| 6 | reserved (MUST be 0) |
| 7 | reserved (MUST be 0) |

A receiver that observes any reserved bit set **MUST** refuse the event with a typed error.

### 6.3 Wire-format size constraints

A receiver **MUST** refuse any wire-format input that is not exactly 16 bytes. The CBOR-decoder used by the reference implementation enforces a maximum depth of 8 and a maximum length of 256 bytes; both are compile-time constants verified by Kani harness `cbor_decoder_bounded`.

### 6.4 Byte order

All multi-byte fields are little-endian. Implementations **MUST** convert to host byte order before interpreting fields.

---

## 7. Cryptographic verification

### 7.1 Authentication is mandatory

Every wire-format consent event **MUST** be verified against the trusted-path public key before the state transition is admitted. An event whose signature does not verify **MUST** be refused with a typed error and **MUST NOT** cause any state change.

### 7.2 Two-stage signature check

The 4-byte `sig_truncated` field is a fast integrity check. The full 64-byte Ed25519 signature is verified out-of-band against the trusted-path public key (the key is stored in the device's secure element — see §8.3).

| Stage | Purpose | Cycles |
|:---|:---|:---:|
| `sig_truncated` check | Constant-time integrity (rejects ~ 99.99999% of accidental corruption) | ≤ 40 |
| Full Ed25519 verification | Cryptographic authentication | ≈ 350,000 (ATECC608B-assisted: ≤ 12 ms wall-clock) |

The 4-byte truncated tag has collision resistance of ~ 2^32 against **accidental** corruption only. It is **NOT** an authentication; it is purely an integrity check used to short-circuit unnecessary full-signature verifications.

### 7.3 Constant-time verification

The full Ed25519 verification path **MUST** be constant-time with respect to the signature value.

**Evidence, stated precisely.** This requirement is **not** currently backed by a
proof and must not be cited as L1. Constant-time execution is a timing and
side-channel property; Kani is a bounded model checker over Rust MIR and cannot
establish it. The harness `signature_verification_constant_time` proves a
narrower, purely functional claim: that the branchless comparison
`ct_eq_u32(a, b)` returns the same result as `a == b` for all inputs. It covers
the 4-byte truncated tag, not the Ed25519 path, and it says nothing about
execution time. Establishing the requirement above needs a timing-aware method
— binary-level analysis, dudect-style statistical testing, or a
secret-independence type system. Until one is applied, §7.3 is an unverified
requirement on implementers.

The reference implementation defers actual point-arithmetic to the [ATECC608B secure element](https://www.microchip.com/en-us/product/ATECC608B), which provides hardware-side-channel-resistant Ed25519. Implementations that perform Ed25519 in software **MUST** use a constant-time implementation (e.g., `ed25519-dalek` with the `zeroize` feature enabled).

### 7.4 Public key management

The trusted-path public key is provisioned at device manufacture and stored in the secure element. Key rotation requires a signed software-update procedure that is outside the scope of this specification; see the device manufacturer's secure-update documentation.

---

## 8. Storage and persistence

### 8.1 Persistence across power cycles

Consent state **MUST** persist across device power cycles. A `Granted` state at shutdown **MUST** be restored as `Granted` at next boot; a `Suspended` state restores as `Suspended`; a `Withdrawn` state restores as `Withdrawn`.

### 8.2 Storage location

Consent state **MUST** be stored in **non-volatile memory** under one of the following:

- The internal Flash of the signal-processing core (e.g., STM32F407 internal Flash).
- The secure element's data zone (e.g., ATECC608B data slots).
- A combination of the two, with the secure element holding the authentication tag of the Flash-stored state.

Storage in external SPI/I²C Flash chips **MUST NOT** be used without an attached authentication tag verified by the secure element on read.

### 8.3 Tamper detection

If on boot the consent state's authentication tag fails to verify, the implementation **MUST** default to `Withdrawn` for the affected manifest, emit an audit event, and refuse to deliver observations until the user installs a fresh manifest through the trusted path.

A failed authentication is treated as a hostile-modification event, not a recoverable error.

### 8.4 Storage of manifest data

The application manifest itself (capability set, rate ceiling, Ed25519 application key) is stored alongside the consent state under the same authentication discipline. A manifest whose authentication tag fails to verify on boot **MUST** be deleted, not merely refused.

---

## 9. Kernel interlock

### 9.1 The interlock contract

The consent state machine interlocks with three other kernel subsystems:

1. **IPC publication path.** The producer side of every SPSC ring buffer (one per application observation stream) reads the consent state before publishing each *IntentObservation*. A `Suspended` or `Withdrawn` state suppresses publication.

2. **SDK error path.** Suppressed publications produce typed `ConsentSuspended` (`0x05`) or `ConsentWithdrawn` (`0x06`) errors per [STANDARD §7.4](https://github.com/AxonOS-org/axonos-standard/blob/main/STANDARD.md#section-7), delivered to the application through the SDK's normal error mechanism.

3. **Cognitive Hypervisor (optional).** On deployments running the Cognitive Hypervisor (TrustZone-M Secure World), StimGuard reads the consent state through a Secure Monitor Call before allowing any stimulation pulse. A `Withdrawn` state immediately disables stimulation regardless of pending pulse-queue content. The path from `Withdrawn` transition to StimGuard's awareness is bounded at ≤ 100 µs (L1).

### 9.2 Atomicity

The transition to `Withdrawn` and the suppression of pending publications **MUST** appear atomic from the perspective of any observer on the application core. An observer **MUST NOT** see an `IntentObservation` followed by a `Withdrawn` error from a moment earlier in monotonic time.

The reference implementation achieves this by writing the new state under `Ordering::SeqCst` before publishing the corresponding error, with a release barrier between the two.

---

## 10. Conformance

### 10.1 Conformance criteria

An implementation is **conformant with the AxonOS Consent v0.5.0 baseline profile** if, and only if, it satisfies all of:

1. Models consent as the three-state FSM of §2.
2. Admits exactly the five transitions of §3.1 and refuses all others.
3. Satisfies the cycle bound of §4.1 (≤ 1648 cycles, analytical).
4. Satisfies the wall-clock bound of §4.2 (≤ 10 ms wall-clock from withdrawal to stream termination).
5. Implements the trusted-path requirements of §5.
6. Implements the wire format of §6 bit-exactly.
7. Implements two-stage cryptographic verification per §7.
8. Persists state across power cycles per §8 with tamper detection per §8.3.
9. Maintains the kernel interlock contract of §9.
10. Passes the conformance test suite shipped with the reference implementation.

An implementation additionally conforms to the **multi-party profile** if it
also satisfies every requirement of §12. The multi-party profile is optional;
the baseline profile is unaffected by it. A v0.4.0-conformant implementation is
a conformant v0.5.0 baseline-profile implementation without modification.

### 10.2 Test vectors

A set of canonical wire-format vectors is published in [`vectors/`](./vectors/). The vector set covers:

- Each of the five admissible transitions.
- Each of the inadmissible transitions, asserted refusal.
- Boundary cases for wire format (under-length, over-length, reserved bits set).
- Signature-failure cases.

A conformant implementation **MUST** produce the documented response for each vector.


### 10.3 Fuzz and differential testing (informative)

The reference implementation is exercised by a coverage-guided fuzz suite (`fuzz/`, built on libFuzzer through `cargo-fuzz`). Three targets search for inputs that would violate this specification:

- **`wire_decode`** drives §6 wire-format decoding with arbitrary byte buffers and asserts that the decoder is *total* — every input yields either an accepted event or a typed refusal, and never a panic or an out-of-bounds read.
- **`roundtrip`** asserts that the §6 encoding is *canonical* — every accepted 16-byte buffer re-encodes to itself, so no two distinct buffers denote one consent event.
- **`fsm_sequence`** drives the §2–§3 state machine with arbitrary streams of correctly-signed events and asserts the four machine invariants: no panic, every stored state valid, `Withdrawn` terminal per §3.3, and every accepted transition admissible per §3.1.

Fuzzing complements, and does not replace, the bounded-model-checking harnesses. The Kani harnesses are **L1 evidence** — an exhaustive proof over a bounded input space. The fuzz suite is **L2-class evidence** — a large, coverage-guided sample of the unbounded input space. The two are run together: the harnesses prove the bounded core, the fuzz suite searches the remainder. The fuzz suite is executed in continuous integration on every change; a discovered crash fails the build and is treated as a specification or implementation defect.

This subsection is informative. It describes the evidence held for the reference implementation; it does not add a conformance obligation on independent implementations beyond those of §10.1.

---

## 11. Threat model

### 11.1 In scope

The consent specification defends against:

- **Honest-but-buggy applications** that fail to honour a software flag.
- **Out-of-band consent changes** (operator, clinician) that an application would not see.
- **In-flight data races** at the moment of withdrawal.
- **Forged application-layer events** claiming to be trusted-path transitions.

### 11.2 Out of scope

The consent specification does **NOT** defend against:

- An attacker who can replace the kernel image (defeat: Secure Boot).
- An attacker who can physically replace the hardware between the user and the trusted path (defeat: device tamper detection at the hardware level).
- Side-channel inference of cognitive state from observable application behaviour unrelated to the consent system (defeat: see [STANDARD §13](https://github.com/AxonOS-org/axonos-standard/blob/main/STANDARD.md#section-13) for what is out of scope of the Standard).
- An honest user who installs a malicious application that legitimately declares its capabilities and then exfiltrates the data it is permitted to receive (defeat: not the consent system's job — applications are end-user software, and what they do with permitted observations is the application's responsibility).

### 11.3 The trust anchor

The consent system's trust anchor is the **kernel** and the **trusted-path public key** stored in the secure element. If either is compromised, the consent system offers no guarantee. The Cognitive Hypervisor and Secure Boot subsystems address compromise of these trust anchors at the hardware level; the consent specification assumes their guarantees hold.

---

## 12. Multi-party (guardian) co-authorisation

*This section is **optional**. An implementation that does not support
multi-party deployments is conformant without it. An implementation that
**does** support multi-party deployments **MUST** satisfy this section.*

### 12.1 Motivation

In clinical deployments a guardian may co-authorise consent decisions together
with the patient — for example in the ALS rehabilitation pilot in the canonical
Standard's roadmap. Multi-party support adds a second authorising key without
weakening the single-party guarantees of §2–§11.

### 12.2 Parties

A multi-party deployment recognises exactly two parties:

| Party | Key | Discriminant |
|:---|:---|:---:|
| Patient | trusted-path (primary) key | `0x01` |
| Guardian | secondary key | `0x02` |

Each consent event **MUST** be verified against the key of the party that
claims to have produced it. An event claimed for one party but signed with the
other party's key **MUST** be rejected with the same error as any other
signature failure (§7).

### 12.3 The safe-direction principle

A transition is **exposure-reducing** if its target is `Suspended` or
`Withdrawn`, and **exposure-increasing** if it is `Suspended → Granted`.
`Granted → Granted` is idempotent and counts as neither.

- Either party **MAY** apply any exposure-reducing transition unilaterally.
  Stopping or pausing the flow **MUST NOT** require the other party.
- An exposure-increasing transition (`Suspended → Granted`) **MUST NOT** take
  effect on the authorisation of a single party. It **MUST** be authorised by
  **both** parties.

This is the central safety property of multi-party operation: the system can
always be stopped by one party, and can only be resumed by two.

### 12.4 Co-authorisation window

An implementation **MUST** define a finite co-authorisation window. When one
party authorises an exposure-increasing transition, the matching authorisation
from the other party **MUST** arrive within the window, measured by the kernel
monotonic timestamp (§6), for the transition to commit. A counter-authorisation
that is older than the window, or that predates the first authorisation, **MUST
NOT** commit; it **MAY** instead be treated as a fresh first authorisation from
the arriving party. The reference implementation's default window is two
minutes.

### 12.5 Terminal state is unaffected

`Withdrawn` remains terminal under multi-party operation. No combination of
party authorisations may transition out of `Withdrawn`; the anti-coercion
property of §3 holds unchanged.

### 12.6 Reference implementation

The reference implementation provides this section's semantics through the
`dual_control` module (`DualControlMachine`, `Party`, `CoAuthOutcome`). The
party-distinctness requirement of §12.3 is verified by the Kani harness
`co_authorisation_requires_two_parties`.

---

## 13. References

### 13.1 Normative

- **[AxonOS-Standard]** AxonOS Project. *The AxonOS Standard, version 1.0.0.* 2026. CC-BY-SA-4.0. https://github.com/AxonOS-org/axonos-standard
- **[RFC2119]** Bradner, S. *Key words for use in RFCs to Indicate Requirement Levels.* RFC 2119, 1997.
- **[RFC8174]** Leiba, B. *Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words.* RFC 8174, 2017.
- **[RFC8949]** Bormann, C. & Hoffman, P. *Concise Binary Object Representation (CBOR).* RFC 8949, 2020.
- **[Ed25519]** Bernstein, D. J. *Ed25519: high-speed high-security signatures.* 2011.

### 13.2 Informative

- **[Kani]** Kani Verification Project. https://model-checking.github.io/kani/
- **[cargo-fuzz]** Rust Fuzzing Authority. *cargo-fuzz: a `cargo` subcommand for fuzzing with libFuzzer.* https://github.com/rust-fuzz/cargo-fuzz
- **[ATECC608B]** Microchip Technology Inc. *ATECC608B CryptoAuthentication Device Datasheet.*
- **[TrustZone-M]** ARM Limited. *ARMv8-M Architecture Reference Manual* — TrustZone-M chapter.
- **[Capabilities]** Dennis, J. B. & Van Horn, E. C. *Programming Semantics for Multiprogrammed Computations.* CACM 9(3):143–155, 1966.

---

## Authorship and licensing

**Author:** Denis Yermakou. Singapore.

**Specification text:** Released under [CC-BY-SA-4.0](./LICENSE-CC-BY-SA).
**Reference code:** Released under [Apache-2.0 OR MIT](./LICENSE).

This is a solo specification of the AxonOS Project. There are no external co-authors. The historical sequence of versions on this repository may include earlier drafts attributing collaborations that are not part of this specification; v0.3.0 supersedes all such drafts. Where v0.3.0 disagrees with an earlier draft on the consent semantics, v0.3.0 wins.

Cite as:

> Yermakou, D. (2026). *AxonOS Consent Specification, version 0.4.0.* AxonOS Project, Singapore. CC-BY-SA-4.0. https://github.com/AxonOS-org/axonos-consent

A BibTeX entry is available in [`docs/citation.bib`](./docs/citation.bib).

---

**End of SPEC.md.**

Singapore · Zurich · Berlin · Milano · San Mateo
