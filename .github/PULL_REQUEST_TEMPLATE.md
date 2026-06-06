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
- [ ] `python3 tools/verify_consent_repository.py` checks pass
- [ ] Formal harnesses re-run if the FSM / wire / crypto surface changed
- [ ] Conformance vectors regenerated and `sha256sum -c` verified if the wire format changed
- [ ] `CHANGELOG.md` updated under `## [Unreleased]`
- [ ] Version impact considered (the crate is `0.y.z`; breaking changes called out)
- [ ] Security impact reviewed
- [ ] Privacy impact reviewed — **no real neural data**, synthetic only
- [ ] No secrets, keys, or private data included
