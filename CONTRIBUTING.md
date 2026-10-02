# Contributing to axonos-consent

Thank you for considering a contribution. `axonos-consent` is the reference
implementation of the AxonOS consent state machine and trusted-path semantics
(Sections 15 and 16 of the [AxonOS Standard](https://github.com/AxonOS-org/axonos-standard)).
It is safety-relevant infrastructure, so the bar for changes is deliberately high.

## Before you start

- Read [`SPEC.md`](./SPEC.md) — the normative specification this crate implements.
- Read [`docs/SECURITY-MODEL.md`](./docs/SECURITY-MODEL.md) and
  [`docs/privacy-boundary.md`](./docs/privacy-boundary.md).
- For anything that changes a normative requirement, open an issue first; such a
  change is governed upstream in the Standard and may require a specification
  amendment before the implementation can follow.

## Development environment

This is a `#![no_std]`, `#![forbid(unsafe_code)]` crate. The minimum supported
Rust version is **1.85**.

```bash
rustup toolchain install stable
cargo build --all-features
```

## The local gate

Every change must pass, locally, the same checks CI enforces:

```bash
cargo fmt --all --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test --all-features
cargo test --no-default-features          # no_std surface, no dependencies
python3 tools/check.py                    # versions, vectors, SPDX, claims, links, badges
```

For changes to the state machine, the wire format, or the cryptographic surface,
run the formal harnesses as well:

```bash
cargo kani --no-default-features                              # the ten proofs in src/proofs.rs
RUSTFLAGS="--cfg loom" cargo test --release --lib loom        # the gate under every interleaving
cargo +nightly fuzz run auth_forgery -- -max_total_time=60    # and frame_decode, fsm_sequence
```

If you change the wire format or the admissible-transition set, regenerate and
re-verify the conformance vectors:

```bash
cargo run --example gen_vectors --features std     # rewrite vectors/
python3 tools/check.py --write                     # rewrite vectors/SHA256SUMS
```

## Evidence discipline

This project grades every quantitative claim by evidence level: **L1** for a
machine-checked proof, **L2** for a measurement on reference hardware, **L3** for
an independent reproduction. Do not introduce a numeric claim without stating its
level and linking the artefact from which it can be re-checked, and do not label
as measured (L2) anything whose trace is not published. The crate currently makes
no L3 claim, and pull requests must not add one.

## Cognitive-data rule

Never include real neural data anywhere in the repository — not in examples, not
in tests, not in issues, not in logs. Examples and fixtures use synthetic data
only. See [`docs/privacy-boundary.md`](./docs/privacy-boundary.md).

## Commits and pull requests

- Use clear, conventional commit subjects (`fix:`, `feat:`, `docs:`, `ci:`,
  `security:`, `chore(release): vX.Y.Z`).
- Update [`CHANGELOG.md`](./CHANGELOG.md) under a new `## [Unreleased]` heading
  for any user-visible change.
- Keep the public API stable; the crate is `0.y.z` and the surface is not yet
  locked, but breaking changes must be called out explicitly in the pull request.
- Fill in the pull-request template, including the security and privacy review
  items.

## Reporting security issues

Do **not** open a public issue or pull request for an exploitable vulnerability.
See [`SECURITY.md`](./SECURITY.md) and email **security@axonos.org**.

## Questions

General questions: **connect@axonos.org**.

<sub>© 2026 Denis Yermakou · The AxonOS Project · connect@axonos.org</sub>
