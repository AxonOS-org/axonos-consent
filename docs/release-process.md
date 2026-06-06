# Release process

Releases are cut from `main`, tagged `vX.Y.Z`, and published automatically by
`.github/workflows/release.yml`, which extracts the matching `CHANGELOG.md`
section. The crate is `0.y.z`: the public surface is not yet locked, and a
`1.0.0` release will accompany a second independent implementation.

## Versioning

Strict [SemVer](https://semver.org/). For this pre-1.0 crate: a minor bump
(`0.Y+1.0`) accompanies a backward-compatible addition; a patch bump
(`0.Y.Z+1`) accompanies a fix or editorial change. The version in `Cargo.toml`
and `CITATION.cff`, the `CHANGELOG.md` entry, the tag, and the GitHub Release
title must all agree.

## Pre-tag gate

Run, and require green:

```bash
cargo fmt --all --check
cargo clippy --all-features --all-targets -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo test --doc --all-features
for c in repository-contract line-counts cargo-manifest readme-surface \
         lib-surface ci-workflow ci-job-count contact-policy \
         clean-public-surface overclaim-guard standard-mapping \
         security-policy changelog license-files source-tree \
         path-hygiene rustdoc-hygiene; do
  python3 tools/verify_consent_repository.py "$c" || exit 1
done
cd vectors && sha256sum -c SHA256SUMS && cd ..
cargo kani        # for changes to the FSM, wire format, or crypto surface
```

## Cutting the release

1. Bump `version` in `Cargo.toml` and `CITATION.cff`.
2. Move the `## [Unreleased]` items into a new `## [X.Y.Z] — YYYY-MM-DD` entry.
3. Commit (`chore(release): vX.Y.Z`), run the gate, then tag and push:

```bash
git tag -a "vX.Y.Z" -m "AxonOS Consent vX.Y.Z"
git push origin main --tags
```

`release.yml` then creates the GitHub Release from the changelog section. Do not
hand-create a release that the changelog does not document.
