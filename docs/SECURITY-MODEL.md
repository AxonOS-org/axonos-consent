# Security model

What `axonos-consent` protects, from whom, how, and on what evidence. The
normative threat model is [SPEC §11](../SPEC.md#11-threat-model); this page is
the engineer's view of it.

## Assets

1. **The consent decision** — whether a manifest's observations may flow.
2. **Its timeliness** — a withdrawal must take effect at once, not eventually.
3. **Its permanence** — a withdrawal must not be undone through the same
   installation.

## Adversaries

| Adversary | Can | Cannot |
|:--|:--|:--|
| Network or IPC attacker | deliver arbitrary bytes to the consent path; knows the public keys | sign with a trusted-path key |
| Replayer | resend any frame ever seen, before or after a reboot | change a signed byte |
| Relabeller | present a guardian frame as the patient's, or the reverse | re-sign it |
| Clock manipulator | choose the timestamp inside a frame it signs | move the kernel's clock |
| Fault | corrupt the stored state, or the gate word | — |
| Application | request anything through the SDK | reach the trusted path |

## Four layers, four properties

| Layer | Property | Evidence |
|:--|:--|:--|
| Wire | Decoding is total; an accepted record has exactly one encoding | Kani; fuzz `frame_decode` |
| Authentication | No field of an unauthenticated record influences anything but its own refusal | Kani; 768-bit flip test; fuzz `auth_forgery` |
| Sequence and FSM | No frame is admitted twice; `Withdrawn` is absorbing; corruption reads as `Withdrawn` | Kani; tests |
| Publication gate | No observation is committed after a withdrawal is stored | loom; real-thread test |

The boundaries between the layers are where 0.8.0 failed: a public-key tag let
bytes cross from the wire layer to the state machine unauthenticated, and a
check-then-publish interlock let an observation cross a withdrawal. In 0.9.0
each boundary is a type or an atomic: only an `Authenticated` record reaches
the state machine, and only a successful compare-and-swap on the gate word
publishes.

## Assumptions

- `ed25519-dalek` implements RFC 8032 verification correctly, including the
  strict checks. It is tested here against RFC 8032 keys and the conformance
  vectors, not proven.
- The trusted-path signing keys are held as SPEC §5.2 requires, and the kernel
  image is the one secure boot measured.
- The kernel's monotonic clock does not run backwards. If it ever reads earlier
  than an arming instant, a pending co-authorisation is treated as stale.
- Persistent storage is authenticated as SPEC §8.3 requires.

## Not addressed here

Side channels on the signing device; key provisioning and rotation; the misuse
of observations an application was entitled to receive. Each has its owner in
the Standard.

<sub>© 2026 Denis Yermakou · The AxonOS Project · security@axonos.org</sub>
