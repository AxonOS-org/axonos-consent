# Release process

Releases are cut from `main`, tagged `vX.Y.Z`, and published by
`.github/workflows/release.yml`, which takes the body of the GitHub Release from
the matching `CHANGELOG.md` section. Nothing is released that the changelog does
not describe.

## Versioning

Strict [SemVer](https://semver.org/). Below 1.0.0, a minor bump marks a breaking
change and a patch bump a compatible one. Three versions move together and
`tools/check.py` refuses a build in which they disagree:

| | Where |
|:--|:--|
| Crate | `Cargo.toml`, `CITATION.cff`, `docs/citation.bib`, the top `CHANGELOG.md` entry, `README.md` |
| Specification | the header of `SPEC.md`, `SPEC_VERSION` in `src/lib.rs`, `docs/citation.bib`, `README.md` |
| Wire format | `MAGIC` and `FRAME_LEN` in `src/wire.rs`, SPEC §6 |

## Pre-tag gate

```bash
cargo fmt --all --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo run --example gen_vectors --features std -- --check
python3 tools/check.py
RUSTFLAGS="--cfg loom" cargo test --release --lib loom
cargo kani --no-default-features
```

CI runs all of it, and more, on every push; the tag is pushed only when `main`
is green.

## Cutting the release

1. Move the unreleased notes into `## [X.Y.Z] — YYYY-MM-DD`, and bump every
   version the table above lists.
2. Commit, push `main`, wait for CI.
3. Tag and push:

```bash
git tag -a "vX.Y.Z" -m "axonos-consent vX.Y.Z"
git push origin "vX.Y.Z"
```

4. Check that the Release exists, is marked latest, and carries the changelog
   section.

<sub>© 2026 Denis Yermakou · The AxonOS Project · connect@axonos.org</sub>
