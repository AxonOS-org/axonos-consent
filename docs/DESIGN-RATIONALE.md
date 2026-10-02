# Design rationale

Why the crate is built the way it is. Each section is a decision, the
alternatives it beat, and the reason.

## 1. A fixed binary frame, not a general-purpose encoding

The record is 32 bytes at fixed offsets. A general-purpose encoding would add a
decoder — depth limits, length limits, integer widths, alternative
representations — to the most security-critical input of the system, and every
alternative representation is a second way to encode a decision a signature has
already committed to. A fixed layout has no second encoding, which Kani proves
in one harness.

## 2. Ed25519, strict

Ed25519 is small, fast, deterministic and widely implemented, including in
secure elements. Strict verification refuses non-canonical scalars and
small-order points, so a signature cannot be malleated into a second valid
frame, and a small-order key cannot become a trust anchor. The magic `AXC2` at
the start of every signed message keeps a consent signature from being valid
anywhere else, including under a later wire version.

## 3. A verifier trait, and a type no one else can build

Deployments differ in where verification happens: in software on the
application processor, or in a secure element. The `SignatureVerifier` trait
lets either be plugged in. What must not differ is whether verification
happens, so the machines accept only `Authenticated`, which only the crate can
construct, after the verifier says yes.

## 4. Sequence numbers, not timestamps

A timestamp asks the receiver to trust the sender's clock, and clocks drift,
reset and lie. A sequence number asks only that the signer count. The receiver
keeps the last number per signer and refuses anything not greater. An authentic
frame spends its number even when its transition is refused: otherwise a frame
that is inadmissible now could be held back and replayed into a state in which
it is admissible.

## 5. One word for consent and publication

A separate state variable and ring index can only be coordinated with a lock or
with a window in which they disagree. Packing both into one `AtomicU32` makes a
withdrawal and a commit two operations on one atomic object, which the memory
model orders totally. The cost is a 30-bit count and a power-of-two ring of at
most 2^29 slots; the gain is a linearization point that needs no lock and no
proof beyond the atomic itself — and loom checks it anyway.

## 6. Fail closed

A state byte that is not `Granted` or `Suspended` reads as `Withdrawn`. The
alternative — trapping on corruption — turns a fault into a crash in the middle
of the kernel's publication path. Withdrawing is always safe; publishing never
is unless consent is certain.

## 7. Dual control: the role in the signature, the window on the kernel's clock

If the caller says which party a frame is from, the caller can be wrong or
lying; so the party is a signed bit. If one key serves both parties, one signer
can co-authorise alone; so the keys must differ. If the window runs on the
frames' timestamps, the signers choose the window; so it runs on the kernel's
monotonic clock. A pending request is not persisted: after a reboot, both
parties authorise again.

## 8. Dependencies

With the default `ed25519` feature the crate depends on `ed25519-dalek` 3 and
what it pulls in; without it, on nothing. `ed25519-dalek` 3 sets the MSRV at
1.85. Its precomputed tables are opt-in (`ed25519-fast`), because flash on the
target is scarcer than cycles.

<sub>© 2026 Denis Yermakou · The AxonOS Project · connect@axonos.org</sub>
