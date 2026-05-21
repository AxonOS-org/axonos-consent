# Design rationale (informative)

This document explains **why** the consent subsystem is designed the way it is.

---

## 1. Why not a Boolean

A Boolean consent toggle cannot model "I want to pause temporarily" vs "I
want to terminate this session permanently". Conflating them forces either:

- Treating every pause as a termination (annoying — user has to re-grant on
  every break).
- Treating every termination as a pause (dangerous — coercion is undefeated).

Three states is the minimum that gets both right. Adding more states
(`Investigating`, `LimitedScope`, etc.) was considered and rejected:
each addition multiplies the transition graph and the verification effort,
without a corresponding gain in user-meaningful expressiveness.

## 2. Why `Withdrawn` is terminal

The non-reversibility of `Withdrawn` is the most contentious design choice.
Several alternatives were considered:

- **Reversible withdrawal**: simpler UX, but defeats the anti-coercion
  guarantee. An attacker (or a stressed clinician) could pressure the user
  to "unwithdraw"; the kernel could not distinguish coerced reversal from
  genuine resumption.
- **Time-bounded withdrawal**: after, e.g., 24 hours, automatically reset.
  This was rejected because the timer is itself a kernel-controlled clock,
  and an attacker with kernel access could set it arbitrarily.
- **Multi-step revocation**: require N consecutive withdrawal events. This
  reduces accidents but does not defend against coercion.

Terminal `Withdrawn` requiring a fresh manifest install through the trusted
path is the only design that resists coercion. The UX cost (re-install) is
real but acceptable for a safety-critical action.

## 3. Why 16-byte wire format

See [ARCHITECTURE.md §2](./ARCHITECTURE.md#2-the-16-byte-wire-format).

## 4. Why two-stage signature check

The truncated-tag check exists to short-circuit unnecessary full Ed25519
verifications. The full check costs ~ 12 ms wall-clock on the ATECC608B;
running it on every event regardless of validity would dominate the
withdrawal latency budget.

The two-stage design lets the fast 40-cycle integrity check filter out
~ 99.99999% of accidentally-corrupted events before the slow path runs.
The slow path runs once per accepted event.

## 5. Why CBOR-compatible wire format

The 16-byte record is CBOR-decodable at the data-model level (an array of
five integer-typed elements). This means a generic CBOR decoder can read
the record alongside other CBOR-encoded kernel metadata, simplifying the
trusted-path partition's code budget.

CBOR was chosen over Protobuf or FlatBuffers because:

- CBOR has a public specification (RFC 8949).
- CBOR has fixed depth/length bounds that are Kani-verifiable.
- CBOR has multiple `#![no_std]` Rust implementations.

## 6. Future work

### 6.1 Scoped consent

Different capabilities could be in different consent states for the same
manifest (e.g., `Navigation` granted but `WorkloadAdvisory` suspended).
This is reserved for a future v0.4 or v0.5 of the specification.

### 6.2 Multi-party consent

For clinical deployments where a guardian co-authorises with a patient, the
consent state machine needs a second-signature path. Out of scope for v0.3.0;
reserved for v0.5 or v1.0 of the specification.

### 6.3 L3 evidence

The first L3 (independent lab) validation is targeted for the Phase 1 pilot
described in the canonical Standard's [ROADMAP.md](https://github.com/AxonOS-org/axonos-standard/blob/main/ROADMAP.md).
