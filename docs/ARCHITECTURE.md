# Architecture (informative)

This document is the informative companion to [SPEC.md](../SPEC.md). It explains
**how** the consent subsystem is designed and implemented, while the SPEC defines
**what** any conformant implementation must do.

If this document disagrees with SPEC, SPEC wins.

---

## 1. The three structural choices

### 1.1 FSM, not Boolean

A two-state Boolean (on/off) consent system cannot represent the difference
between "the user paused temporarily" (resumable) and "the user revoked"
(terminal). Conflating the two would either (a) lose pause-as-an-option, or
(b) lose the anti-coercion guarantee of `Withdrawn` being terminal. The
three-state FSM is the minimum that preserves both.

### 1.2 Kernel-level, not application-level

Consent at the application level fails in the four ways enumerated in SPEC §11
(application bugs, late updates, in-flight data, out-of-band changes).
Lifting consent below the application layer is what makes withdrawal
enforceable rather than advisory.

### 1.3 Trusted path is hardware-coupled

If the consent event could be synthesised by software, the kernel could not
distinguish a user revocation from an application masquerading. The trusted
path therefore couples to hardware: either a physical button on a discrete GPIO
line, or a Secure-World UI partition under TrustZone-M. Both are
software-unforgeable from the Normal World.

---

## 2. The 16-byte wire format

The 16-byte size was selected to fit exactly within one half of a 32-byte ARM
cache line, ensuring that one consent event fits in a single coherency unit.
A larger record would either span two cache lines (introducing torn-read
hazards) or waste space; a smaller record could not carry enough state to be
useful.

Byte budget:

- 1 byte state, 1 byte flags = 2 bytes (discriminant + bits)
- 2 bytes manifest_id (16 bits, supporting 64k concurrent installs)
- 8 bytes timestamp_us (64 bits, ~584,000 years at 1 µs)
- 4 bytes sig_truncated (fast integrity)

= 16 bytes total.

The trade-off: 4 bytes of integrity check is collision-resistant against
accidental corruption (~ 2^32) but is **not** an authentication. The full
64-byte Ed25519 verification is invoked out-of-band by the trusted-path
crypto path before admission.

---

## 3. Why constant-time signature verification

A timing side channel on signature verification could leak the trusted-path
key over many tries. The constant-time property is enforced by:

- The `ct_eq_u32` function uses XOR + arithmetic operations only, no branches.
- The full Ed25519 path runs on the ATECC608B secure element on the reference
  hardware, which has hardware-side-channel resistance certified at the chip
  level.

L1 evidence comes from Kani harness `signature_verification_constant_time`.

---

## 4. Why `#![no_std]`

The kernel is `#![no_std]`. The consent subsystem must live in the same
linker domain. A heap allocator on the consent path would break the bounded-
execution property — heap allocation is unbounded in general.

`#![forbid(unsafe_code)]` is also set. The crate compiles to a binary with no
`unsafe` blocks; memory safety is structural.

---

## 5. Verification methodology

Four Kani harnesses provide L1 evidence:

- `handle_withdraw_terminates` — bounded cycles per transition.
- `fsm_no_invalid_transitions` — admissibility predicate is correct.
- `cbor_decoder_bounded` — wire-format size enforcement.
- `signature_verification_constant_time` — no branches on signature value.

L2 evidence comes from soak runs on the reference hardware. The
[claims catalogue](https://github.com/AxonOS-org/axonos-standard/blob/main/validation/claims.md)
in the canonical Standard documents each measurement.
