# Changelog

All notable changes to `axonos-consent` are documented here. Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); semantic versioning per [SemVer 2.0.0](https://semver.org/spec/v2.0.0.html).

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
