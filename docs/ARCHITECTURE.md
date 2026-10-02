# Architecture

`axonos-consent` turns a signed frame from the trusted path into a consent
decision, and makes that decision the one thing every observation must pass.
This page shows how the pieces fit; [SPEC.md](../SPEC.md) is normative, and
[DESIGN-RATIONALE.md](./DESIGN-RATIONALE.md) explains the choices.

## 1. Where it sits

```text
 trusted path ──signed frame──▶ kernel ───────────────────────────────────────────┐
 (button, Secure-World UI)        │                                               │
                                  ▼                                               │
                        ConsentMachine::handle()                                   │
                                  │ admits, then stores the state                 │
                                  ▼                                               │
 signal pipeline ──observation──▶ PublicationGate::try_publish() ──▶ SPSC ring ──▶ SDK ──▶ application
                                  │                                       ▲
                                  └─ refused: 0x05 suspended · 0x06 withdrawn ┘
```

The application never talks to the machine. It can ask the person to use the
trusted path; it cannot use it for them.

## 2. Modules

| Module | Responsibility | Holds state? |
|:--|:--|:--:|
| `wire` | Split a frame, decode and validate the record, encode a record | no |
| `auth` | Verify the signature under the named party's key; mint `Authenticated` | no |
| `state` | The three states, the seven admissible pairs, failing closed | no |
| `gate` | Consent state and publication count in one `AtomicU32` | yes |
| `machine` | Single-party admission: manifest, sequence, transition | yes |
| `dual_control` | Two parties, the safe direction, the window on the kernel's clock | yes |
| `proofs` | Kani harnesses, compiled only under `cfg(kani)` | — |

The pure modules can be read, tested and proved in isolation; the stateful ones
are small compositions of them.

## 3. Admitting a frame

`ConsentMachine::handle` runs the checks of SPEC §7.6 in order: shape (wire),
signature (auth), manifest, sequence, transition (state). Only after all of them
does it store the new state, with one atomic write into the gate. A transition
needs `&mut self`, so transitions are serialized by the borrow checker; the
gate is read through `&self`, so the publication path never waits for one.

## 4. Publishing an observation

The producer of each SPSC ring follows a two-step protocol:

1. write the observation into slot `gate.published() % capacity`;
2. call `gate.try_publish()`. `Ok` makes the slot visible to the consumer;
   `Err(Suppressed)` means consent is not granted, and the slot is abandoned.

The consumer reads `gate.published()` with acquire ordering and reads only
committed slots. Because the state and the count are one word, a withdrawal and
a commit are totally ordered: there is no instant at which a withdrawn state
and a fresh commit can coexist. `loom` checks this under every interleaving.

## 5. Persistence

After each admitted frame, the kernel writes `machine.persisted()` — the state
and the last sequence from each signer — atomically, before acknowledging the
trusted path. At boot it rebuilds the machine with `restore`. A pending
co-authorisation is deliberately not persisted.

## 6. Bringing your own verifier

With the `ed25519` feature off, the crate has no dependencies, and the
integrator supplies a `SignatureVerifier` — typically a secure element. The
trait asks for strict RFC 8032 verification and a key check; the type system
ensures that whatever verifier is supplied is consulted on every frame.

<sub>© 2026 Denis Yermakou · The AxonOS Project · connect@axonos.org</sub>
