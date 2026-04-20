# Security Policy

`axonos-consent` is the reference runtime for consent enforcement in safety-critical brain-computer interface applications. Security vulnerabilities here can contribute directly to patient-safety hazards downstream. Reports are taken seriously.

## Scope

This policy applies to:

- The `axonos-consent` crate on crates.io.
- The `axonos-consent` source at https://github.com/AxonOS-org/axonos-consent.
- Published documentation and examples.

Out of scope:

- The MMP base protocol (report to SYM.BOT at https://sym.bot).
- Downstream applications built against this crate (report to their maintainers).

## Reporting

**Do not file public GitHub issues for security reports.**

Email: `axonosorg@gmail.com`
Subject: `Security: axonos-consent <one-line summary>`

Please include:

1. Affected version (`cargo pkgid axonos-consent`).
2. Feature flags in use (`default`, `alloc`, `json`, `std`, `stim-guard`).
3. A minimal reproducer or written description.
4. Your assessment of impact: does this affect
   - **Wire-level parsing** (CBOR / JSON decoder robustness)?
   - **State machine integrity** (illegal transitions, WITHDRAWN escape)?
   - **Hardware enforcement path** (StimGuard / DAC gate)?
   - **Availability** (panic, deadlock, DoS on resource exhaustion)?
5. Whether you have a fix in mind.

Acknowledgement within 3 business days (Singapore time). If no reply after 5 business days, please email again.

## What we commit to

- **Acknowledgement** within 3 business days.
- **Triage** within 10 business days: confirmed reproduction or a written not-a-vulnerability explanation.
- **Coordinated disclosure window** up to 90 days from acknowledgement. For issues affecting downstream medical-device integrations, we may request up to 180 days with the reporter's agreement.
- **Credit** in the advisory unless you prefer anonymity.
- **A fix** in a patch release, cross-referenced in `CHANGELOG.md`.

## What we cannot commit to

- A paid bug bounty. Enterprise support customers receive formal SLAs; see https://axonos.org/enterprise.html.
- Support for unmaintained versions. Security fixes land on the latest minor; previous minors get fixes at our discretion for 6 months.

## Threat model (summary)

The AxonOS consent runtime has these safety properties by design:

1. **Bounded wire parsing.** The CBOR decoder enforces `MAX_MAP_FIELDS = 8`, `MAX_STRING_LEN = 128`, `MAX_NESTING_DEPTH = 4`, and explicit rejection of CBOR major types 1, 2, 4, 6, 7. A malformed or hostile input cannot cause unbounded resource consumption.

2. **Exhaustive state machine.** The 3×3 state transition table is compiler-enforced. Attempting to transition from WITHDRAWN to any other state always produces `Err(Transition)`. There is no wildcard arm that could permit an undefined transition in a future code change.

3. **Zero allocation in the default build.** The default `no_std` build has no heap. All fixed-size buffers are bounded at compile time.

4. **Hardware enforcement is separated from protocol logic.** The `DacGate` trait defines the hardware interlock; the trait binding is enabled only when the `stim-guard` feature is set. The runtime cannot enforce hardware state change unless the downstream integrator explicitly wires the trait.

Reports describing violations of any of these properties are treated as critical.

Out of this model's scope:

- Vulnerabilities in `serde_json` or other third-party dependencies. Report upstream; we will coordinate.
- Physical-access attacks on the target device (physical custody is part of the trusted computing base).
- Vulnerabilities in the MMP base protocol or other implementations (see Scope above).

---

`axonos.org · medium.com/@AxonOS · axonosorg@gmail.com`
