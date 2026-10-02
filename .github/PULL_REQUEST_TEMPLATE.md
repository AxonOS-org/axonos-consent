## Summary

<!-- What does this change do, and why? -->

## Type of change

- [ ] Documentation
- [ ] Bug fix
- [ ] Feature
- [ ] Security hardening
- [ ] Conformance / specification alignment
- [ ] Release maintenance

## Checklist

- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --all-features --all-targets -- -D warnings` passes
- [ ] `cargo test --all-features` and `--no-default-features` pass
- [ ] `python3 tools/check.py` passes
- [ ] `cargo kani --no-default-features` and the loom models re-run if the wire, auth, FSM or gate changed
- [ ] Vectors regenerated (`gen_vectors`, `tools/check.py --write`) if the wire format or an outcome changed
- [ ] `CHANGELOG.md` updated under `## [Unreleased]`
- [ ] Version impact considered (the crate is `0.y.z`; breaking changes called out)
- [ ] Security impact reviewed
- [ ] Privacy impact reviewed — **no real neural data**, synthetic only
- [ ] No secrets, keys, or private data included
