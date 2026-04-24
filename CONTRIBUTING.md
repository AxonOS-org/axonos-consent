# Contributing to axonos-consent

Thank you for your interest. This is the reference hard real-time runtime for the MMP Consent Extension; contributions are evaluated against the safety-critical standard of the project.

## Before you start

Two things to understand before sending a PR:

1. **Wire format and state machine are defined by the specification** (MMP Consent Extension v0.1.0, authored by SYM.BOT, CC-BY-4.0). This crate implements the specification; it does not define it. Changes to this crate that would cause it to diverge from the published specification are out of scope — open an issue to discuss test coverage instead. Changes that affect the specification itself must be raised with SYM.BOT, not here.

2. **`#![forbid(unsafe_code)]` is non-negotiable.** We do not accept PRs that introduce `unsafe` blocks. If you hit a place where you think `unsafe` is required, open an issue describing the constraint — there is almost always a safe way to express it.

## Quick start

```sh
git clone https://github.com/AxonOS-org/axonos-consent
cd axonos-consent

# Run the test suite (default no_std build).
cargo test

# Full JSON codec + interop vectors.
cargo test --features json

# no_std target build.
cargo build --target thumbv7em-none-eabihf --no-default-features

# Lints.
cargo clippy --all-features -- -D warnings

# Formatting.
cargo fmt --check

# Fuzz (nightly required).
cargo +nightly fuzz run fuzz_cbor_decode -- -max_total_time=60
```

All of the above must pass before a PR will be reviewed.

## Scope of contribution

**Happy to accept:**

- Bug fixes with a failing test added.
- Performance improvements that preserve the published WCET bounds (show the numbers).
- Additional fuzz targets.
- Documentation improvements.
- Additional `#[derive(Debug)]` / `impl` on public types where missing.
- New AxonOS-range reason codes (`0x10` – `0xFF`) for concrete safety scenarios, with spec-section references.

**Please discuss first:**

- Anything touching the state machine or wire format (goes through the spec).
- Adding a new feature flag.
- Adding a runtime dependency (the no_std default build is currently zero-dep).
- Changing the WCET bounds in documentation.

**Will not be merged:**

- PRs that introduce `unsafe`.
- PRs that remove the bounded CBOR decoder limits (map fields, string length, nesting depth).
- PRs that add wildcards to the 3×3 state transition table — exhaustiveness is a compile-time guarantee and we keep it that way.
- PRs that add heap allocation to the no_std default build.

## Code style

- `#[must_use]` on constructors and `Result`-returning public functions where appropriate.
- `#[non_exhaustive]` on public enums that may grow.
- Prefer `const fn` where possible.
- Every public item gets a doc comment with at least one sentence. Non-trivial items get `# Example` and `# Errors` sections where applicable.

## Tests

- Every bug fix: add a regression test.
- Every new public function: add a unit test.
- Wire-format changes: the 15 canonical vectors (defined by the specification) must continue to pass byte-for-byte. The vector file in this repository is reproduced from the specification and should not be modified here — any vector updates come from SYM.BOT as part of specification revisions.

## Security issues

See [`SECURITY.md`](./SECURITY.md). Do not open public issues for security reports.

## Licensing

By submitting a PR you agree your contribution is dual-licensed Apache-2.0 / MIT, the same as the crate.

## Questions

General technical questions → GitHub Discussions or the Medium comment threads at https://medium.com/@AxonOS.
Commercial / enterprise questions → `axonosorg@gmail.com`.

---

`axonos.org · medium.com/@AxonOS · axonosorg@gmail.com`
