# AxonOS Consent Specification

**Version 0.6.0** · 2026-10-02 · Normative

**Author:** Denis Yermakou
**Project:** The AxonOS Project
**Licence:** [CC-BY-SA-4.0](./LICENSE-CC-BY-SA) (specification text) · [Apache-2.0 OR MIT](./LICENSING.md) (reference code) · [CC0-1.0](./vectors/LICENSE) (conformance vectors)

---

## Preface

This document specifies the **AxonOS Consent** subsystem: the kernel-level state machine that decides whether a manifest's intent observations may flow, and the protocol by which the trusted path changes that decision. It elaborates Sections 15 (consent state semantics), 16 (the trusted path) and 20 (the error taxonomy) of the [AxonOS Standard](https://github.com/AxonOS-org/axonos-standard), and weakens none of them.

The reference implementation is the `axonos-consent` crate. Version 0.9.0 of the crate implements version 0.6.0 of this specification.

**What changed in 0.6.0.** Version 0.5.0 authenticated a consent event with a 4-byte tag computed from the record and the trusted-path *public* key. A public key is not a secret, so the tag authenticated nothing, and the full signature the text required was never carried on the wire. Version 0.6.0 replaces the wire format with version 2, which carries a 64-byte Ed25519 signature; makes replay protection normative, as Standard §16 already demanded; requires that the consent check and the publication of an observation be one atomic step; binds the dual-control role into the signed record; and moves the co-authorisation window onto the kernel's clock. The full list is in [Appendix A](#appendix-a-changes-from-050). The defect is recorded as advisory [AXC-2026-001](./docs/advisories/AXC-2026-001.md).

---

## Document conventions

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY** and **OPTIONAL** are to be interpreted as described in BCP 14 ([RFC 2119](https://www.rfc-editor.org/rfc/rfc2119), [RFC 8174](https://www.rfc-editor.org/rfc/rfc8174)) when, and only when, they appear in all capitals.

Byte-level definitions are normative. Rust names are informative and refer to the reference implementation. Evidence levels (L1, L2, L3) are those of Standard Section 22.

---

## Contents

1. [Scope](#1-scope)
2. [The consent state machine](#2-the-consent-state-machine)
3. [Admissible transitions](#3-admissible-transitions)
4. [Timing](#4-timing)
5. [Trusted path](#5-trusted-path)
6. [Wire format](#6-wire-format)
7. [Authentication](#7-authentication)
8. [Storage and persistence](#8-storage-and-persistence)
9. [Kernel interlock](#9-kernel-interlock)
10. [Conformance](#10-conformance)
11. [Threat model](#11-threat-model)
12. [Multi-party (guardian) co-authorisation](#12-multi-party-guardian-co-authorisation)
13. [Evidence](#13-evidence)
14. [References](#14-references)
- [Appendix A. Changes from 0.5.0](#appendix-a-changes-from-050)

---

## 1. Scope

### 1.1 In scope

This specification defines the three consent states and the transitions between them; the trusted path through which transitions are requested; the wire format of a consent frame; how a frame is authenticated and protected against replay; what must persist across power cycles; how the consent decision gates the publication of observations; and, optionally, how a second party co-authorises decisions.

### 1.2 Out of scope

The acquisition, processing or transport of neural signal; the capability system and the manifest format (Standard Sections 12 and 14); the user interface of the trusted path; the provisioning and rotation of keys; secure boot. Each is a dependency of this specification, not a subject of it.

---

## 2. The consent state machine

### 2.1 The three states

| State | Discriminant | Meaning |
|:--|:--:|:--|
| `Granted` | `0x01` | Intent observations flow. |
| `Suspended` | `0x02` | Observations do not flow; a consumer receives the consent-suspended error, `0x05`. Resumable through the trusted path. |
| `Withdrawn` | `0x03` | Observations never flow again for this installation; a consumer receives the consent-withdrawn error, `0x06`. Terminal. |

Discriminants `0x00` and `0x04`–`0xFF` are reserved. A receiver **MUST** refuse a frame that carries one (§7.6).

### 2.2 Initial state

A freshly installed manifest **MUST** start in `Granted`, with no sequence number consumed from any signer.

### 2.3 Representation and failing closed

An implementation **MUST** read a stored state that is not `0x01` or `0x02` — whether `0x03`, a reserved value, or the product of corruption — as `Withdrawn`. Corruption may stop observations; it **MUST NOT** start them.

---

## 3. Admissible transitions

### 3.1 The transition graph

Exactly seven ordered pairs are admissible: the three identities, and four transitions that change the state.

| From \ To | `Granted` | `Suspended` | `Withdrawn` |
|:--|:--:|:--:|:--:|
| `Granted` | identity | pause | withdraw |
| `Suspended` | resume | identity | withdraw |
| `Withdrawn` | — | — | identity |

### 3.2 Inadmissible transitions

`Withdrawn → Granted` and `Withdrawn → Suspended` are inadmissible. A frame that requests one **MUST** be refused (§7.6) and **MUST NOT** change the state.

### 3.3 Non-reversibility of `Withdrawn`

`Withdrawn` is absorbing. Observations resume only through the installation of a new manifest, which is a new installation with a new manifest identifier and a fresh state machine. This is the anti-coercion property: a person who has withdrawn cannot be made to re-grant through the same installation.

### 3.4 Idempotency

An identity transition is admitted and changes nothing but the consumed sequence number (§7.5). It lets the trusted path re-assert a state without first reading it.

---

## 4. Timing

### 4.1 The transition

The state-machine step that follows authentication — the manifest check, the sequence check, the admissibility check and the store — **MUST** execute in constant time with respect to its inputs: it **MUST NOT** loop, allocate or block. The reference implementation's step is straight-line code over fixed-width integers.

No cycle bound is specified for the step at this revision. Releases up to 0.8.0 of the reference implementation published one, derived for a path that verified the 4-byte tag of §A; that path no longer exists, and the figure is withdrawn. A bound will be specified only together with its derivation and an on-device measurement at evidence level L2.

### 4.2 Authentication

Ed25519 verification (§7) dominates the cost of admitting a frame. Its duration depends on the verifier — software, or a secure element — and **MUST** be accounted for in the deployment's withdrawal budget (§4.3).

### 4.3 The withdrawal budget

A `* → Withdrawn` transition **MUST** stop all observation flow for the manifest within **10 ms** of the trusted path emitting the withdrawal frame. The budget covers delivery of the frame to the kernel, authentication, the transition, and the stop of publication. Under §9 the stop of publication is immediate: once the withdrawal is stored, no observation can be committed. This budget is a requirement on deployments; the reference implementation does not claim to meet it on any specific hardware.

---

## 5. Trusted path

### 5.1 Definition

The **trusted path** is the channel through which consent transitions are requested, ending at a signing key that the application layer cannot use. A consent frame whose signature does not verify under a trusted-path key **MUST** be refused.

### 5.2 Acceptable trusted paths

A conformant implementation **MUST** use one of:

- a **physical control** wired to a component that holds the signing key and that the application core cannot drive;
- a **Secure-World UI** on an ARM TrustZone-M device, holding the signing key in the Secure World;
- an equivalent channel whose signing key the application layer **provably cannot** use.

### 5.3 Application-layer requests are refused

The kernel **MUST NOT** convert an application-layer request — a network message, an IPC message from the application core, a file, an environment variable — into a consent frame. An application can ask the user to act on the trusted path; it cannot act for them.

### 5.4 Audit

Every admitted frame **SHOULD** be recorded in a tamper-evident log readable by the device operator and not by the application. An entry **MUST** include the kernel's monotonic time of admission (Standard Section 11), the manifest identifier, the from- and to-states, the signer, the sequence number, and a cryptographic hash of the frame.

---

## 6. Wire format

### 6.1 The frame

A consent frame is exactly **96 bytes**: a 32-byte **record**, followed by the 64-byte Ed25519 **signature** over those 32 bytes.

| Offset | Size | Field | Encoding | Rule |
|--:|--:|:--|:--|:--|
| 0 | 4 | `magic` | ASCII `AXC2` | Protocol `AXC`, wire version `2`. Any other value **MUST** be refused. |
| 4 | 1 | `state` | `u8` | A discriminant of §2.1. |
| 5 | 1 | `flags` | `u8` | §6.2. |
| 6 | 2 | `manifest_id` | `u16` LE | The installation the frame is for. |
| 8 | 8 | `sequence` | `u64` LE | §7.5. |
| 16 | 8 | `timestamp_us` | `u64` LE | The signer's clock at signing, in µs. Informational (§6.3). |
| 24 | 8 | `reserved` | zero | Any non-zero byte **MUST** be refused. |
| 32 | 64 | `signature` | RFC 8032 | Ed25519 over bytes 0–31 (§7). |

A receiver **MUST** refuse any input that is not exactly 96 bytes.

### 6.2 Flags

| Bit | Name | Rule |
|:--:|:--|:--|
| 0 | `terminal` | **MUST** be set if, and only if, `state` is `Withdrawn`. A record in which they disagree **MUST** be refused. |
| 1 | `from-secure-world` | Set when the frame originated in a Secure-World UI. Signed and audited; it does not change admission. |
| 2 | — | Reserved. In wire version 1 this bit meant *replay-tolerant*; it is retired, because every frame is now subject to §7.5. |
| 3 | `guardian` | Set when the signer is the guardian (§12). Clear for the patient. |
| 4–7 | — | Reserved. |

A record with any reserved bit set **MUST** be refused.

### 6.3 The signer's timestamp

`timestamp_us` records the signer's clock for the audit log. A receiver **MUST NOT** use it to decide admission, ordering, freshness or any co-authorisation window; it is authenticated, but it is still the sender's claim. Ordering is the job of `sequence` (§7.5), and time is the job of the kernel's clock (§12.4).

### 6.4 Byte order

Every multi-byte integer is little-endian.

### 6.5 Canonical encoding

Every 32-byte record that a receiver accepts **MUST** re-encode to exactly the same 32 bytes. There is no second encoding of any record, so a signature commits to exactly one decision.

---

## 7. Authentication

### 7.1 Authentication is mandatory

Every frame **MUST** be authenticated before any field of its record influences anything other than the frame's own refusal. A frame that fails authentication **MUST** be refused with code `0x08` and **MUST NOT** change the state, the consumed sequence numbers, or anything else.

### 7.2 The algorithm

The signature is **Ed25519** as defined in [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032) §5.1, PureEdDSA, over the 32 bytes of the record. The magic `AXC2` at the start of every signed message separates this protocol, and this version of it, from any other use of the same key.

### 7.3 Strict verification

A verifier **MUST** refuse:

- a signature whose scalar `S` is not reduced — that is, `S ≥ L`, where `L` is the order of the prime-order subgroup (RFC 8032 §5.1.7);
- a signature whose point `R` is not canonically encoded or has small order;
- a signature that does not satisfy the verification equation.

Verification handles only public data, so this specification places no constant-time requirement on it. The signer is a different matter (§11.2).

### 7.4 Trust anchors

A receiver **MUST** refuse, when it is configured, a trusted-path public key that does not decode to a curve point, or that decodes to a point of small order. Under §12 it **MUST** also refuse a configuration in which the patient and guardian keys are equal.

### 7.5 Replay protection

Each signer keeps a `sequence` that **MUST** increase strictly with every frame it signs for a manifest, and **MUST** persist across the signer's own power cycles.

A receiver keeps, per manifest and per signer, the last sequence number it consumed, initially zero. After a frame authenticates and names this manifest, the receiver **MUST** refuse it with code `0x08` unless its `sequence` is strictly greater than the last one consumed from its signer. A frame that passes this check **MUST** consume its sequence number — the receiver records it as the last one consumed — **whether or not** the transition it requests is then admitted. An authentic frame refused for its transition can therefore never be held back and replayed into a later state.

### 7.6 Order of checks

A receiver **MUST** apply the checks in this order, and refuse a frame for the first rule it breaks:

| # | Check | Refusal | Code |
|--:|:--|:--|:--:|
| 1 | Length is exactly 96 bytes | `WireFormatLength` | `0x07` |
| 2 | Magic is `AXC2` | `BadMagic` | `0x07` |
| 3 | `state` is a discriminant of §2.1 | `ReservedDiscriminant` | `0x07` |
| 4 | No reserved flag bit is set | `ReservedFlagBit` | `0x07` |
| 5 | `terminal` agrees with `state` | `TerminalFlagMismatch` | `0x07` |
| 6 | The reserved bytes are zero | `ReservedFieldNonZero` | `0x07` |
| 7 | The signature verifies under the key of the party the record names | `SignatureInvalid` | `0x08` |
| 8 | `manifest_id` names this installation | `ManifestMismatch` | `0xFF` |
| 9 | `sequence` exceeds the last consumed from this signer | `Replay` | `0x08` |
| 10 | The transition is admissible from the current state | `InadmissibleTransition` | `0xFF` |

Checks 1–6 read only the shape of the frame. Nothing after check 7 trusts a field that the signature has not covered. A record that names the guardian is refused at check 7 by a receiver that has no guardian key.

---

## 8. Storage and persistence

### 8.1 What persists

An implementation **MUST** persist, per manifest: the consent state, and the last sequence number consumed from each signer. A state restored at boot **MUST** be the state stored at the last admitted frame. Losing the sequence numbers would re-open every old frame to replay, so they persist with the same discipline as the state.

### 8.2 When it persists

The state and the sequence numbers **MUST** be written as one atomic unit, after a frame is admitted and before the admission is acknowledged to the trusted path.

### 8.3 Where it persists

Consent data **MUST** be stored in non-volatile memory that the application core cannot write: internal Flash of the signal-processing core, the data zone of a secure element, or a combination in which the secure element holds an authentication tag over Flash-stored data. External Flash **MUST NOT** be used without such a tag, verified on every read.

### 8.4 Tamper detection

If the authentication tag over stored consent data fails to verify at boot, the implementation **MUST** treat the manifest as `Withdrawn`, record the event in the audit log, and deliver no observations for it until a new manifest is installed through the trusted path.

### 8.5 Pending co-authorisations

A pending co-authorisation (§12.4) **MUST NOT** persist. After a power cycle both parties authorise again, which errs in the safe direction.

---

## 9. Kernel interlock

### 9.1 The contract

The consent state gates every publication of an intent observation for its manifest. A publication refused because consent is `Suspended` or `Withdrawn` produces the consent-suspended (`0x05`) or consent-withdrawn (`0x06`) error, delivered through the SDK's normal error path.

### 9.2 Linearization

Reading the consent state and then publishing is a race: a withdrawal can land between the read and the write. An implementation **MUST** therefore make the consent check and the commit of a publication **one atomic step**, ordered with respect to every state change. The required property:

> Once a transition to `Suspended` or `Withdrawn` has been stored, no publication commits until the state is `Granted` again — and after `Withdrawn`, none ever does.

Equivalently, every committed publication is ordered before the withdrawal, and is therefore one the person had consented to.

### 9.3 The reference design

The reference implementation keeps the consent state and the count of committed publications in one 32-bit atomic word: the state in bits 0–1, the count, modulo 2³⁰, in bits 2–31. A producer writes an observation into the next slot of its ring and then commits it with a compare-and-swap that succeeds only while the state bits read `Granted`; the consumer reads the count with acquire ordering and reads only committed slots. A state change rewrites the state bits of the same word. Because both are operations on one atomic object, they are totally ordered, and §9.2 holds by construction. A word whose state bits are `00` reads as `Withdrawn` (§2.3). Ring capacity **MUST** be a power of two no larger than 2²⁹ under this design.

### 9.4 Stimulation

On deployments with a stimulation path, the stimulation guard **MUST** read the consent state through the same linearization point before every pulse, and `Withdrawn` **MUST** disable stimulation regardless of the content of any pending pulse queue.

---

## 10. Conformance

### 10.1 Criteria

An implementation **conforms to the baseline profile of AxonOS Consent 0.6.0** if, and only if, it:

1. models consent as the three-state machine of §2, failing closed per §2.3;
2. admits exactly the seven pairs of §3.1;
3. meets the timing requirements of §4;
4. implements the trusted path of §5;
5. implements wire version 2 of §6 bit-exactly;
6. authenticates every frame per §7, with strict verification, trust-anchor checks, replay protection, and the order of checks of §7.6;
7. persists per §8;
8. provides the linearization of §9.2;
9. produces the documented outcome for every conformance vector (§10.2).

It conforms to the **multi-party profile** if it also satisfies §12.

### 10.2 Conformance vectors

The [`vectors/`](./vectors/) directory holds twenty vectors, each a frame and the outcome it must produce: the state and sequence the frame meets, and either the resulting state or the refusal and its code. Every frame is signed with a key from RFC 8032 §7.1, so the set can be checked without the reference implementation. The set covers the seven admissible pairs and both inadmissible ones, each shape refusal of §7.6, a forged signature, a signature under the wrong key, a replayed sequence, a non-canonical signature (`S + L`), a wrong manifest, and a guardian record presented to a single-party receiver.

### 10.3 Fuzzing (informative)

The reference implementation runs three coverage-guided fuzz targets on every change: `frame_decode` (§6: total and canonical decoding), `fsm_sequence` (§2, §3, §7.5, §9: invariants under arbitrary record streams, with authentication stubbed out so the search reaches the state machine), and `auth_forgery` (§7: no frame the fuzzer builds is admitted under a real key). Each runs for 60 seconds in continuous integration — a regression net rather than a search. Fuzzing complements the proofs of §13; it does not replace them.

---

## 11. Threat model

### 11.1 In scope

This specification defends against:

- **forged frames** from anyone who does not hold a trusted-path signing key, including anyone who holds the public key;
- **replayed frames**, before or after a power cycle;
- **relabelled frames** — a guardian frame presented as the patient's, or the reverse;
- **manipulated timestamps** — a signer's clock moved to stretch or shrink a co-authorisation window;
- **malleated signatures** — a second encoding of a valid signature;
- **the withdrawal race** — an observation published in the instant consent is withdrawn;
- **corrupted state** — a stored or in-memory state byte damaged by a fault;
- **applications** that fail to honour a software flag, or that try to request consent changes themselves.

### 11.2 Out of scope

This specification does not defend against compromise of the trust anchors of §11.3; physical replacement of the trusted-path hardware; side channels on the signing device, which belong to the trusted path; implementation defects in the Ed25519 library, which the reference implementation takes from `ed25519-dalek`; or an application that legitimately receives observations and misuses them, which is the domain of the capability system (Standard Section 12).

### 11.3 Trust anchors

The kernel image, and the trusted-path signing keys held where §5.2 requires. If either is compromised, this specification offers no guarantee; secure boot and the secure element defend them.

---

## 12. Multi-party (guardian) co-authorisation

*Optional. An implementation that supports multi-party deployments **MUST** satisfy this section.*

### 12.1 Motivation

In clinical deployments a guardian may share consent decisions with the patient. A second key adds that party without weakening anything in §2–§11.

### 12.2 Parties

| Party | Key | `guardian` flag |
|:--|:--|:--:|
| Patient | the trusted-path key | 0 |
| Guardian | a second key | 1 |

The party is declared by the `guardian` flag inside the signed record, and the frame **MUST** be verified under that party's key; a relabelled frame fails verification. The two keys **MUST** differ (§7.4). Each party has its own sequence (§7.5).

### 12.3 The safe-direction principle

A transition to `Suspended` or `Withdrawn`, and every identity transition, **MAY** be applied by either party alone, and cancels any pending co-authorisation. The exposure-increasing transition, `Suspended → Granted`, **MUST NOT** take effect on the authority of one party; it requires both. The system can always be stopped by one party and resumed only by two.

### 12.4 The co-authorisation window

When one party requests `Suspended → Granted`, the request is **pending**, armed at the kernel's monotonic time of receipt. It completes when the **other** party requests the same transition while the kernel's monotonic time is within the window of the arming instant. The window **MUST** be finite and non-zero; the reference default is two minutes. A matching request that arrives late, or from the same party, **MUST NOT** complete the transition; it re-arms the request from the arriving party. Windows **MUST** be measured on the kernel's clock and never on `timestamp_us` (§6.3).

### 12.5 Terminal state

`Withdrawn` remains terminal. No combination of authorisations leaves it.

### 12.6 Reference implementation

`DualControlMachine` in the `dual_control` module. Kani proves that two frames from the same party can never resume, and that one key cannot serve as both parties.

---

## 13. Evidence

| Requirement | Property | Evidence | Level |
|:--|:--|:--|:--:|
| §6.5 | Decoding is total; accepted records are canonical | Kani `wire_decode_is_total_and_canonical`; fuzz `frame_decode` | L1 |
| §2.3 | Corruption reads as `Withdrawn` | Kani `stored_state_fails_closed`; exhaustive unit test | L1 |
| §3.3 | `Withdrawn` is absorbing | Kani `withdrawn_is_absorbing`, `withdrawn_machine_stays_withdrawn`, `gate_withdrawal_is_absorbing` | L1 |
| §7.1 | No change without authentication | Kani `no_transition_without_authentication`; 768-bit flip test; fuzz `auth_forgery` | L1 |
| §7.5 | No sequence admitted twice | Kani `no_sequence_is_admitted_twice`; power-cycle test | L1 |
| §9.2 | Commit only while `Granted` | Kani `gate_publishes_only_while_granted` | L1 |
| §9.2 | No commit after a withdrawal is stored | loom: three models, every interleaving with at most three preemptions; real-thread test | model-checked |
| §12.3 | One party cannot resume | Kani `one_party_cannot_resume` | L1 |
| §12.2 | Distinct keys | Kani `one_key_cannot_be_both_parties` | L1 |
| §7.3 | Strict verification | Conformance vectors 13, 14 and 19; tests | test |
| §7.2 | Ed25519 correctness | `ed25519-dalek`; RFC 8032 keys | assumed |
| §4 | Timing | — | none claimed |

The Kani proofs replace the signature verifier with one that answers `false`, `true` or either, so they hold whatever the real verifier decides; the verifier's own correctness is the one assumption, recorded in the last rows. No L2 or L3 claim is made at this revision.

---

## 14. References

### 14.1 Normative

- [AxonOS Standard](https://github.com/AxonOS-org/axonos-standard), Sections 11, 15, 16, 20 and 22.
- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119) and [RFC 8174](https://www.rfc-editor.org/rfc/rfc8174), key words.
- [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032), Edwards-Curve Digital Signature Algorithm (EdDSA).

### 14.2 Informative

- `ed25519-dalek` and `curve25519-dalek`, dalek-cryptography.
- The Kani Rust Verifier, the model-checking project.
- loom, the Tokio project: a model checker for concurrent Rust.

---

## Appendix A. Changes from 0.5.0

| Area | 0.5.0 | 0.6.0 |
|:--|:--|:--|
| Wire format | 16-byte record; a 4-byte tag computed from the public key; the full signature "out of band" | Version 2: a 96-byte frame — a 32-byte record and its 64-byte Ed25519 signature (§6) |
| Authentication | None that a public-key holder could not reproduce | Strict RFC 8032 verification before any field is trusted (§7) |
| Replay | Not addressed | Per-signer sequence, consumed on every authentic frame, persisted (§7.5, §8) |
| Terminal flag | Not checked against the state | Must agree with the state (§6.2) |
| Bit 2 of flags | *replay-tolerant* | Retired, reserved |
| Domain separation | None | Magic `AXC2` inside the signed message (§7.2) |
| Data model | Described as compatible with a general-purpose encoding with depth and length bounds | A fixed binary layout with no second encoding (§6.5) |
| Stored state | A corrupted byte trapped | Fails closed to `Withdrawn` (§2.3) |
| Interlock | Read the state, then publish | Check and commit as one atomic step (§9.2) |
| Dual control | Party asserted by the caller; window on the signer's timestamp | Party signed in the record; keys must differ; window on the kernel's clock (§12) |
| Timing | A cycle figure for the tag path | Withdrawn; requirements only (§4) |
| Evidence | Five harnesses, not compiled into the crate | Ten Kani proofs and three loom models, run in CI (§13) |

---

## Authorship and licensing

**Author:** Denis Yermakou, The AxonOS Project.

The specification text is released under [CC-BY-SA-4.0](./LICENSE-CC-BY-SA); the reference code under [Apache-2.0 OR MIT](./LICENSING.md); the conformance vectors under [CC0-1.0](./vectors/LICENSE).

To cite this specification:

> Yermakou, D. (2026). *AxonOS Consent Specification, version 0.6.0.* The AxonOS Project. CC-BY-SA-4.0. https://github.com/AxonOS-org/axonos-consent

<sub>© 2026 Denis Yermakou · The AxonOS Project · connect@axonos.org</sub>
