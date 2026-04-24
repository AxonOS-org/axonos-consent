# Changelog

All notable changes to `axonos-consent` are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versioning: [SemVer 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2] — 2026-04-12

### Added
- `WithdrawAllResult` struct returned by `withdraw_all()` — includes `[Option<PeerId>; MAX_PEERS]` + count for WCET-safe audit trail. Zero-alloc discipline preserved.
- `#![warn(missing_docs)]`, `#![warn(missing_debug_implementations)]`, `#![warn(rust_2018_idioms)]` at crate root.
- `html_root_url` for docs.rs links.
- Complete Cargo.toml metadata: `homepage`, `documentation`, `include` list.

### Compatibility
- **Wire format unchanged** from v0.1.0. All 15 canonical interop vectors continue to pass.
- **State machine unchanged.** The 3×3 exhaustive FSM is identical.
- v0.2.2 is purely additive — no breaking API changes.

## [0.2.1] — 2026-04-11

### Added
- `stim-guard` feature flag. The `DacGate` trait binding is enabled only when this flag is set; the trait declaration is always present. Aligns the crate with IEC 62304 §5.3 SOUP qualification expectations: the hardware interlock is a user-supplied implementation.

### Changed
- Clippy `-D warnings` pass on all features + targets.

## [0.2.0] — 2026-04-10

### Added
- `ReasonBuf` — 64-byte fixed-size reason buffer, replaces `&'static str` for more flexible reason code payloads while preserving zero-allocation discipline.
- CBOR decoder security hardening: `MAX_MAP_FIELDS = 8`, `MAX_STRING_LEN = 128`, `MAX_NESTING_DEPTH = 4`, bitmask duplicate-key detection, explicit rejection of CBOR major types 1, 2, 4, 6, 7.
- Fuzz targets for CBOR decode and round-trip (`cargo +nightly fuzz run`).
- GitHub Actions CI pipeline with `thumbv7em-none-eabihf` build verification.

### Changed
- Error enum: layered L1–L4 taxonomy (Wire / Struct / State / System).
- `process_raw()` single entry point; internal helpers made private.

## [0.1.0] — 2026-04-04

Initial release. Rust implementation of the MMP Consent Extension v0.1.0 specification (authored by SYM.BOT, CC-BY-4.0).

### Added
- Exhaustive 3×3 state machine (GRANTED / SUSPENDED / WITHDRAWN × Withdraw / Suspend / Resume).
- Consent frame types per §3, reason code registry per §3.4.
- Zero-allocation CBOR codec for local IPC; JSON codec (feature-gated) for relay boundary.
- Invariants module enforcing MUST/SHOULD per §10.
- Single public entry point: `ConsentEngine::process_raw()`.
- `#![no_std]` default build with `#![forbid(unsafe_code)]`.
- 15 canonical interop test vectors, SHA-256 locked.
- Verified against SYM.BOT production relay — 4 consent frames, zero transport errors, silent ignore by non-consent nodes (MMP §7 forward compatibility).
- Dual Apache-2.0 / MIT license.

### Compatibility at initial release
- **MMP Consent Extension v0.1.0:** 15/15 canonical vectors PASS.
- **MMP base protocol v0.2.2:** §3.5, §7, §7.2, §16, §16.4 aligned.

[Unreleased]: https://github.com/AxonOS-org/axonos-consent/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/AxonOS-org/axonos-consent/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/AxonOS-org/axonos-consent/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/AxonOS-org/axonos-consent/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/AxonOS-org/axonos-consent/releases/tag/v0.1.0
