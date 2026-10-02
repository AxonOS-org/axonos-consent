# Changelog

All notable changes to `axonos-consent` are documented here. Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); semantic versioning per [SemVer 2.0.0](https://semver.org/spec/v2.0.0.html).

---

## [0.9.2] — 2026-10-02

### Fixed

- **The `auth_forgery` fuzz target reported a forgery that was not one.** It
  verified frames under the RFC 8032 TEST 1 key, and its seed corpus held
  conformance vector 13: a genuine TEST 1 signature with one bit flipped. In
  0.9.1's CI run libFuzzer flipped the bit back in under half a minute and
  presented vector 01 — genuine, valid, and indistinguishable to the oracle from
  a forgery. The crate behaved correctly; the oracle was satisfiable without
  forging anything. The target now verifies under a key whose secret no one
  holds: a point derived from SHA-512 of a fixed label and multiplied by the
  cofactor. No genuine signature under it exists, so an admitted frame could
  only be a forgery. `tools/check.py` repeats the derivation on every change,
  with its curve arithmetic self-tested against the base point and RFC 8032,
  and a test checks that the reference verifier accepts the key as a trust
  anchor while refusing every conformance vector under it.

The crate is unchanged: `src/` is byte-identical to 0.9.1.

## [0.9.1] — 2026-10-02

### Fixed

- **Three Kani proofs could not complete in 0.9.0's CI run.**
  `no_transition_without_authentication`, `no_sequence_is_admitted_twice` and
  `withdrawn_machine_stays_withdrawn` carried an unwind bound of 4. On their
  path, decoding compared the 4-byte magic as an array, which Kani models as a
  loop needing five unwindings, so each run stopped at an unwinding assertion
  before it reached its property. The magic is now compared as one 32-bit
  integer, which leaves the decode path without a single loop, and every bounded
  harness uses 40, with the reason written beside the bounds in
  `src/proofs.rs`. The properties are unchanged; what changes is that their
  proofs can now finish.
- The spelling job pins `crate-ci/typos` to v1.50.3; the `master` branch it
  followed is deprecated.

No change to behaviour, the wire format, the specification or the conformance
vectors.

## [0.9.0] — 2026-10-02

A security release, and breaking by necessity. Specification 0.6.0, wire format 2.

### Security

- **Fixed [AXC-2026-001](./docs/advisories/AXC-2026-001.md) (critical): consent
  transitions were accepted without authentication.** Through 0.8.0,
  `handle_event()` admitted a frame when a 4-byte tag matched a value computed
  from the record and the trusted-path *public* key, so anyone who knew the key
  could withdraw, suspend or resume at will. Every frame now carries a 64-byte
  Ed25519 signature, verified under strict RFC 8032 rules before any field is
  acted on.
- **Replay protection.** Every frame carries a per-signer sequence number that
  must increase; an authentic frame spends its number even when its transition
  is refused, and the counters persist across power cycles.
- **The withdrawal race is closed.** Consent and the publication count now share
  one atomic word, so the consent check and the commit of an observation are a
  single compare-and-swap. Nothing can be published after a withdrawal is stored.
- **Dual control hardened.** The role is signed into the record, the two keys
  must differ, and the co-authorisation window runs on the kernel's clock rather
  than on a timestamp the sender chose.
- **Fail-closed state.** A corrupted state byte reads as `Withdrawn` instead of
  panicking.

### Breaking

- Wire format 2: a 96-byte frame — a 32-byte record beginning `AXC2`, and its
  Ed25519 signature. Wire format 1 is not accepted.
- `ConsentMachine::new(manifest_id, key, verifier)` returns `Result` and refuses
  small-order keys; `handle_event(ConsentEvent)` is replaced by `handle(&[u8])`.
- `DualControlMachine::propose(&[u8], now_us)` takes the kernel's monotonic time
  and no longer takes a caller-asserted `Party`.
- `ObservationGate` is replaced by `PublicationGate::try_publish`, which makes
  the check and the publication one step.
- Flag bit 2 (*replay-tolerant*) is retired and reserved. The terminal flag must
  agree with the state.
- `ConsentError` gains `BadMagic`, `TerminalFlagMismatch`,
  `ReservedFieldNonZero`, `Replay`, `KeyInvalid` and `ConfigurationInvalid`, and
  loses `CborBoundViolation`.
- MSRV 1.85, for `ed25519-dalek` 3.

### Added

- `auth`: the `SignatureVerifier` trait and the reference `Ed25519Strict`
  (feature `ed25519`, on by default; `ed25519-fast` adds precomputed tables).
  `Authenticated` cannot be constructed outside the crate.
- `gate`: `PublicationGate` and `Suppressed`.
- `machine::Persisted`: what must survive a power cycle.
- Ten Kani proofs in `src/proofs.rs`, compiled into the crate and run in CI as
  a blocking job; three `loom` models of the gate, also blocking.
- Twenty conformance vectors for wire format 2, signed with the RFC 8032 test
  keys, generated by `examples/gen_vectors.rs` and checked byte for byte in CI;
  `tests/vectors.rs` executes them.
- Tests that attack the boundary: public-key-only forgery, all 768 single-bit
  flips, `S + L` malleability, replay across a power cycle, relabelled roles.
- Fuzz targets `frame_decode`, `fsm_sequence`, `auth_forgery`, each run for 60
  seconds in CI.
- `tools/check.py`: versions, wire constants, vectors, SPDX headers, retired
  claims, links and badges, checked on every change.
- `SECURITY.md` as a real policy, and `docs/advisories/`.

### Removed

- The 4-byte truncated tag and the FNV-style function that computed it.
- The five harnesses under `kani/`. They were never compiled into the crate, so
  `cargo kani` could not run them; `src/proofs.rs` replaces them.
- The published cycle figure, derived for the tag path; see SPEC §4.
- The language describing the record as compatible with a general-purpose
  encoding with depth and length bounds. The format is a fixed binary layout.
- `tools/verify_consent_repository.py`, which CI never ran; `tools/check.py`
  replaces it. `clippy.toml`, whose MSRV disagreed with `Cargo.toml`.
- The GitHub template text that stood in for `SECURITY.md`.

### Carried from the evidence correction of 2026-08-16

#### Corrected

- **The ≤ 1648 cycle bound was mistagged `L1 (Kani-proven)`.** It appeared that
  way in `README.md`, in `SPEC.md` §4.1–§4.4, and in the `handle_event()` doc
  comment, and SPEC §4.1 named `handle_withdraw_terminates` as the harness
  backing it. That harness proves termination and target-state correctness and
  contains no cycle assertion; Kani is a bounded model checker over Rust MIR and
  cannot produce a Cortex-M cycle count. The figure is an analytical bound
  derived by instruction counting and is now tagged `analytical` everywhere,
  with the derivation artefact marked pending publication.
- **The harness doc comment claimed coverage it does not have.** It stated
  "≤ 1648 cycles for any starting state". `starting_state` is generated and
  constrained, but `ConsentMachine::new()` always constructs the default state,
  so only the `Granted` transition is exercised. The comment now says so.

- **SPEC §7.3 cited Kani as proving constant-time verification.** It does not,
  and cannot: constant-timeness is a timing/side-channel property outside the
  reach of a bounded model checker over MIR. The harness
  `signature_verification_constant_time` proves only that `ct_eq_u32` is
  functionally equivalent to `==`, and covers the 4-byte truncated tag rather
  than the Ed25519 path. §7.3 is now recorded as an unverified requirement.
- **Two further harness doc comments overstated their scope.**
  `cbor_decoder_bounded` claimed depth bounds it does not assert;
  `fsm_no_invalid_transitions` claimed a reachability result over wire inputs
  when it checks the pure admissibility predicate. Both corrected.
  `co_authorisation_requires_two_parties` was audited and states its scope
  correctly; it is unchanged.

#### Open then, closed by 0.9.0

- Extend `handle_withdraw_terminates` to the `Suspended` and `Withdrawn`
  starting states.
- Cover depth and string-length bounds in `cbor_decoder_bounded`.
- Choose and apply a timing-aware method for the §7.3 constant-time requirement.
- Publish the instruction-count derivation behind the 1648 figure, or withdraw
  the figure.

All four are closed: the harnesses are replaced by ten proofs that compile and
run; the record has no general-purpose encoding left to bound; verification
handles only public data and carries no constant-time requirement; and the cycle
figure is withdrawn. No measured (L2) figure changed.

## [0.8.0] — 2026-06-06

### Added
- **Populated the conformance vector set.** The `vectors/` directory previously
  contained only its `README.md` and `LICENSE`; the twelve vectors the README
  described had never been committed. This release ships all twelve as
  `vector-NN-*.bin` + `vector-NN-*.expected.json`, generated from and byte-for-byte
  consistent with the reference decoder and state machine (`src/wire.rs`,
  `src/state.rs`, `src/error.rs`): five admissible transitions, the two terminal
  refusals, reserved-discriminant and reserved-flag-bit decode failures, the
  undersize/oversize length refusals, and a manifest-mismatch refusal.
- **`vectors/SHA256SUMS`** over every vector file, verifiable with `sha256sum -c`.
- **Foundation-grade contribution surface.** Added `CONTRIBUTING.md`,
  `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1), a pull-request template, and
  structured issue forms (bug report, feature request, security-boundary
  discussion, and a config that routes vulnerabilities to the security policy).
- **Supply-chain workflow.** Added `.github/workflows/security.yml`
  (cargo-deny, cargo-audit, dependency review) and `.github/dependabot.yml` for
  the cargo and github-actions ecosystems.
- **Documentation.** Added `docs/privacy-boundary.md` and
  `docs/release-process.md`; added a `clippy.toml` pinning the MSRV.

### Changed
- **Rewrote `vectors/README.md`** into a precise specification: the 16-byte
  little-endian record layout, the flags mask, the normative decode order, the
  `.expected.json` schema, the canonical test context, the full vector index, and
  a reference harness algorithm.
- Bumped the crate version to 0.8.0, aligning `Cargo.toml` with `CITATION.cff`
  and the changelog, and modernised two repository-verifier checks
  (`cargo-manifest`, `changelog`) to be version-agnostic rather than pinned to
  stale tokens.

### Notes
- No library, public-API, or wire-format change. The vectors describe the
  existing wire format and three-state machine (stable since the v0.3.0
  specification); they are conformance data, not a behavioural change.

## [0.7.0] — 2026-06-04

### Added
- **Automated GitHub Releases.** `.github/workflows/release.yml` now creates a
  GitHub Release on every `v*.*.*` tag push, with the matching CHANGELOG section
  as the body and source archives (`.tar.gz`, `.zip`) attached. Previously,
  pushing a tag produced no Release — v0.6.0 was tagged but never published.

### Changed
- **README de-staled and made self-maintaining.** The crate-version badge is now
  a dynamic `github/v/release` shield that always reflects the latest tag and can
  no longer go stale. The stale "current version" markers — the version-status
  table that still listed v0.4.0 as current, the "v0.5.0 is the current release"
  note, and the footer version — are replaced with pointers to the releases page
  and CHANGELOG (single source of truth). The "Position in the AxonOS stack"
  table now also lists `axonos-protocol`, `axonos-conformance`, and
  `axonos-validation`.

### Notes
- No library, API, or wire-protocol change from v0.6.0. The single-party consent
  protocol remains stable as of the v0.3.0 specification; this release covers
  release automation and documentation only.

## [0.6.0] — 2026-06-04

### Fixed
- CI `integrity` job: restored the top-level `LICENSE` pointer (SPDX
  `Apache-2.0 OR MIT`) required by the multi-licence layout and the integrity
  check. It had been removed during an organisation-wide licence cleanup, which
  broke the `test -f LICENSE` step and, with it, the aggregate CI gate. The full
  `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-CC-BY-SA`, and `vectors/LICENSE`
  files are unchanged.
- CI `fuzz` job: install `cargo-fuzz` as a prebuilt binary via
  `taiki-e/install-action` instead of `cargo install --locked` (which compiled
  it on nightly and could fail), and pin `--target x86_64-unknown-linux-gnu` for
  every `cargo fuzz build`/`run` — the prebuilt binary otherwise targets musl,
  whose `std` is absent and whose static libc is incompatible with the sanitiser.

## [0.5.0] — 2026-05-28

### Added — multi-party (guardian) co-authorisation

The headline feature of this release. For clinical deployments — the ALS
pilot in the canonical Standard's roadmap is the motivating case — a
guardian can now co-authorise consent changes together with the patient.
This implements the second-signature path reserved in
[DESIGN-RATIONALE §6.2](./docs/DESIGN-RATIONALE.md) and specified in the new
[SPEC §13](./SPEC.md).

- **`dual_control` module** with `DualControlMachine`, `Party`
  (`Patient` / `Guardian`), and `CoAuthOutcome` (`Applied` / `PendingCoAuth`).
- **The safe-direction principle.** Either party may *unilaterally reduce*
  exposure — moving to `Suspended` or `Withdrawn` never requires agreement.
  *Increasing* exposure (`Suspended → Granted`, i.e. resuming neural-data
  flow) requires **both** parties to authorise the same transition within a
  bounded window. No sequence of signatures from a single party can resume
  the flow.
- **Per-party signature verification.** A guardian-claimed event is verified
  against the guardian key; a patient-claimed event against the trusted-path
  key. A forged-party event fails with `SignatureInvalid`.
- **Bounded co-authorisation window** (`DEFAULT_CO_AUTH_WINDOW_US`, two
  minutes; configurable via `DualControlMachine::with_window`). A stale or
  out-of-order counter-authorisation never commits; it re-arms a fresh
  pending request instead.
- **`FLAG_GUARDIAN`** (wire flag bit 3) for wire-level disambiguation of
  guardian-originated events. Previously reserved; now defined. This is a
  backward-compatible relaxation — events that were rejected for setting
  bit 3 are now accepted.
- **New Kani harness** `co_authorisation_requires_two_parties` — proves the
  exposure-increasing transition commits only when two *distinct* parties
  authorise it, and that `Suspended → Granted` is the only exposure-increasing
  transition. Brings the formal-proof count to **5 harnesses**.
- **New example** `examples/dual_control.rs` and an extensive `#[cfg(test)]`
  suite (eleven tests) covering unilateral reduction, two-party resume,
  single-party impossibility, window expiry, forged-party rejection, and
  terminal-state immutability.

### Changed

- `SPEC_VERSION` is now `"0.5.0"`; the specification adds §13 (multi-party
  co-authorisation). The three-state single-party machine, the 16-byte wire
  format, and all v0.4.0 timing bounds are **unchanged and byte-compatible**.
- Author email updated to the project-canonical `connect@axonos.org`.
- README brought to the unified AxonOS visual standard (palette badges,
  full stack table, canonical footer).

### Notes

- **Fully additive.** The v0.4.0 single-party API (`ConsentMachine`,
  `ConsentEvent`, `ConsentState`, `ObservationGate`) is unchanged. Existing
  single-device deployments need no changes; dual control is opt-in by
  choosing `DualControlMachine` instead of `ConsentMachine`.
- **Minor bump 0.4.0 → 0.5.0** per SemVer — new functionality, no breaking
  changes, no new runtime dependency (still zero-dependency `no_std`).

---

## [0.4.0] — 2026-05-27

**Verification release. The consent protocol is unchanged from v0.3.0; this release strengthens the evidence base.**

This release adds a coverage-guided fuzz suite to the reference implementation. It changes no protocol semantics: the three-state FSM, the five admissible transitions, the 16-byte wire format, and the timing bounds are byte-for-byte identical to v0.3.0. An implementation conformant with v0.3.0 is conformant with v0.4.0 without modification.

### Added

- `fuzz/` — a coverage-guided fuzz suite built on `cargo-fuzz` / libFuzzer, as a separate non-published crate. Three targets:
  - `wire_decode` — totality of the SPEC §6 wire-format decoder under arbitrary input (never panics, never reads out of bounds).
  - `roundtrip` — canonical-encoding symmetry of the SPEC §6 wire format (no two buffers denote one event).
  - `fsm_sequence` — the SPEC §2–§3 state-machine invariants under arbitrary streams of correctly-signed events (no panic, every stored state valid, `Withdrawn` terminal, every accepted transition admissible).
- `fuzz/README.md` — how to build, run, and triage the fuzz targets.
- `fuzz/corpus/` — a committed seed corpus for all three targets.
- `SPEC.md` §10.3 — "Fuzz and differential testing" — documenting the fuzz suite as L2-class evidence alongside the L1 Kani harnesses.
- A `fuzz` CI job — nightly toolchain; builds all three targets and smoke-runs each for 60 s on every push and pull request. A discovered crash fails the build.

### Changed

- `SPEC.md` bumped to v0.4.0. The bump reflects the new informative §10.3 only; the normative protocol is byte-identical to v0.3.0.
- Crate version `0.3.0` → `0.4.0`; `SPEC_VERSION` in `src/lib.rs` updated accordingly.

### Notes

- No breaking changes. No wire-format change. No new runtime dependency — the crate remains zero-dependency; the fuzz suite is a separate crate that is never published.

---

## [0.3.0] — 2026-05-21

**Solo restart of the consent subsystem under Denis Yermakou's sole authorship.**

This release establishes the AxonOS Consent Specification v0.3.0 as the canonical document, supersedes all prior drafts, and resets the crate to a clean lineage.

### Added

- `SPEC.md` — the canonical, normative consent specification authored solely by Denis Yermakou (12 sections, RFC-2119 conformance keywords throughout, full byte-level wire-format definition).
- `LICENSE-CC-BY-SA` — explicit CC-BY-SA-4.0 license file for the specification text (fixes the previously ambiguous license detection by GitHub).
- `LICENSE` (Apache-2.0 OR MIT dispatcher), `LICENSE-APACHE`, `LICENSE-MIT` — standard Rust crate dual-license pattern for the source code.
- `vectors/LICENSE` — CC0-1.0 dedication for the conformance vectors (public-domain so any conformant implementation can use them freely).
- `docs/ARCHITECTURE.md`, `docs/SECURITY-MODEL.md`, `docs/DESIGN-RATIONALE.md` — informative companions to the normative specification.
- 4 Kani Bounded Model Checking harnesses producing L1 evidence: `handle_withdraw_terminates`, `fsm_no_invalid_transitions`, `cbor_decoder_bounded`, `signature_verification_constant_time`.
- Property-based test suite exercising all 5 admissible × admissible transitions.
- Conformance test vectors in `vectors/` (12 vectors covering admissible transitions, refused transitions, and wire-format edge cases).

### Changed

- The wire format is now the **AxonOS Consent Wire Format v1** — a self-contained 16-byte little-endian record as defined in SPEC §6. There is no external protocol extension.
- The consent state machine is a standalone AxonOS subsystem; the only external dependency is the AxonOS Standard's §6 timing bounds.
- Specification text moved from informal in-tree notes into the canonical `SPEC.md`.

### Removed

- All references to external coupling protocols and external collaborations are removed from the specification, the reference implementation, the test vectors, and the documentation. v0.3.0 is the spec; nothing else.
- Earlier source directories carrying external-protocol coupling code are removed; no functionality is lost because the consent subsystem is, by design, single-device.
- Prior co-authorship attributions in source comments are removed; v0.3.0 is solo-authored.

### Security

- The signature verification path is now L1-verified to be constant-time via the new Kani harness `signature_verification_constant_time`.
- The CBOR decoder's depth and length bounds are now compile-time constants verified by `cbor_decoder_bounded`.
- The 16-byte wire-format size is enforced at the boundary; over-length and under-length inputs are refused at the first read byte, preventing any partial-state observation.

### Compatibility

- **Breaking** with prior 0.x drafts. The wire format, the public API surface, and the type names all changed. Anyone running an earlier 0.x draft must upgrade by re-installing manifests through the trusted path.
- A migration tool from earlier drafts is **not** provided. The prior drafts were pre-public, and clean continuity is preferred over forced compatibility with non-public artefacts.
- Compatibility with the AxonOS Standard v1.0.0 is the only stable interface this release commits to.

### Performance (reference hardware, STM32F407 @ 168 MHz)

| Metric | v0.3.0 | Bound | Evidence |
|:---|---:|---:|:---:|
| Withdrawal cycles (median) | 1098 (6.5 µs) | — | L2 |
| Withdrawal cycles (99.9p) | 1487 (8.85 µs) | — | L2 |
| Withdrawal cycles (worst observed) | 1503 (8.95 µs) | 1648 | L1 + L2 |
| Wall-clock end-to-end termination | 3.2 ms (worst observed) | 10 ms | L2 |
| Soak duration with zero unsafe states | 18 h / 12 × 10⁶ events | — | L2 |

All within bound. No Kani counterexamples.

### Notes

- The repository is now authored solely by Denis Yermakou.
- The specification text is the source of truth; the crate is one conformant implementation of it. Other implementations are welcome and equally legitimate if they pass the conformance vectors.
- Future minor versions (`0.3.x`) will add ergonomic improvements, additional language bindings, and new test vectors. Major-version bumps (`0.4.0`+) are reserved for breaking changes to the public API surface; the **specification** at v0.3.0 is stable across these crate-version bumps.

---

[0.3.0]: https://github.com/AxonOS-org/axonos-consent/releases/tag/v0.3.0
