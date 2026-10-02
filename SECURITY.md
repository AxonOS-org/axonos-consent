# Security policy

`axonos-consent` decides whether a person's neural data may flow. A defect here
is a defect in that person's consent, and is treated that way.

## Supported versions

| Version | Supported | Note |
|:--|:--:|:--|
| 0.9.x | ✓ | Full Ed25519 verification and replay protection |
| ≤ 0.8.x | ✗ | Affected by [AXC-2026-001](./docs/advisories/AXC-2026-001.md): transitions accepted without authentication. Upgrade. |

## Reporting a vulnerability

Email **[security@axonos.org](mailto:security@axonos.org)**, or open a private
report through GitHub's *Report a vulnerability* button on the Security tab. Do
not open a public issue or pull request.

Please include the version or commit, what an attacker can achieve, and the
smallest frame, sequence or program that shows it. A conformance-style vector —
a frame, the state it meets and the outcome — is the most useful form there is.

- **Acknowledgement** within 72 hours.
- **Assessment** within seven days: whether it is a vulnerability, its severity,
  and the plan.
- **Disclosure** coordinated with you, normally within 90 days, published as an
  advisory under [`docs/advisories/`](./docs/advisories/) and on the repository's
  Security tab, with credit unless you prefer otherwise.

## In scope

- The crate: decoding, authentication, the sequence rule, the state machine,
  the publication gate, dual control.
- The specification, where a requirement is unsafe or ambiguous as written.
- The conformance vectors, where one documents the wrong outcome.

Out of scope: vulnerabilities in `ed25519-dalek` itself (report them
[upstream](https://github.com/dalek-cryptography/curve25519-dalek/security);
we will follow and pin), and attacks that require the trust anchors SPEC §11.3
assumes — the kernel image and the trusted-path key.

## Advisories

| ID | Affected | Fixed | Severity | Title |
|:--|:--|:--|:--|:--|
| [AXC-2026-001](./docs/advisories/AXC-2026-001.md) | ≤ 0.8.0 | 0.9.0 | Critical | Consent transitions accepted without authentication |

<sub>© 2026 Denis Yermakou · The AxonOS Project · security@axonos.org</sub>
