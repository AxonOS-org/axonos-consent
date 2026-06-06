# Privacy boundary

AxonOS treats cognitive data as high-sensitivity data. Raw neural signals,
derived cognitive states, intent vectors, and consent transitions must be
handled under explicit capability boundaries.

## What this crate handles

`axonos-consent` is deliberately narrow. It operates on **consent state**
(granted, suspended, withdrawn), a **manifest identifier**, a monotonic
timestamp, and a truncated signature tag. It does **not** acquire, store,
transform, or transport neural signal, and it has no access to intent vectors or
derived cognitive state. The consent decision it computes is the gate that the
kernel consults before such data is permitted to flow elsewhere.

## Rules

- **No real neural data, ever.** Examples, tests, fuzz corpora, and fixtures use
  synthetic data only. Do not attach real recordings to issues or pull requests.
- **No raw signal in logs.** Diagnostic output must not contain signal payloads;
  consent records carry only the fields above.
- **Minimised representations.** Applications built above this crate should
  operate on minimised intent or state representations, not raw signal.
- **Explicit capability grants.** Access is capability-based; a manifest's
  permissions are explicit and bounded.
- **Consent revocation is modelled.** Withdrawal is terminal and bounded in time
  (see `SPEC.md` and Standard Section 15); it is not a soft preference.

## Reporting

Do not place sensitive data in a public issue. Security concerns go to
**security@axonos.org** (see `SECURITY.md`).
