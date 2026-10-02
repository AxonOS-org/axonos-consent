# Conformance vectors — wire format 2

Twenty vectors that make the [AxonOS Consent Specification 0.6.0](../SPEC.md)
executable. Each is a frame and the exact outcome it must produce. An
implementation, in any language, conforms when it produces every outcome below.

## Using a vector

Each vector is two files:

- `vector-NN-name.bin` — the frame as it arrives from the trusted path, usually
  96 bytes;
- `vector-NN-name.expected.json` — what it meets and what must happen.

```json
{
  "manifest_id": 7,
  "trusted_key": "d75a9801…511a",
  "initial_state": "Granted",
  "initial_sequence": 0,
  "result": "refuse",
  "state_after": "Granted",
  "error": "SignatureInvalid",
  "abi_code": "0x08"
}
```

Build a single-party receiver for `manifest_id` with `trusted_key`, in
`initial_state`, with `initial_sequence` already consumed from the patient.
Present the frame. Then:

- `"result": "accept"` — the receiver must admit it, and be in `state_after`;
- `"result": "refuse"` — the receiver must refuse it with `error` (the reason of
  SPEC §7.6) and `abi_code`, and still be in `state_after`, unchanged.

## The keys

Every frame is signed with a key from [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032)
§7.1, so the set can be checked without any AxonOS code:

| Role | RFC 8032 | Public key |
|:--|:--|:--|
| Patient, the trusted path | TEST 1 | `d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a` |
| Any other signer | TEST 2 | `3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c` |

Ed25519 is deterministic: regenerating the set reproduces every byte.

## The set

| # | Vector | Meets | Frame | Outcome |
|--:|:--|:--|:--|:--|
| 01 | `vector-01-granted-to-suspended` | Granted · seq 0 | pause | → Suspended |
| 02 | `vector-02-suspended-to-granted` | Suspended · seq 1 | resume | → Granted |
| 03 | `vector-03-granted-to-withdrawn` | Granted · seq 2 | withdraw while granted | → Withdrawn |
| 04 | `vector-04-suspended-to-withdrawn` | Suspended · seq 3 | withdraw while suspended | → Withdrawn |
| 05 | `vector-05-idempotent-granted` | Granted · seq 4 | re-assert the current state | → Granted |
| 06 | `vector-06-withdrawn-to-granted-refused` | Withdrawn · seq 5 | nothing leaves Withdrawn | InadmissibleTransition · `0xFF` |
| 07 | `vector-07-withdrawn-to-suspended-refused` | Withdrawn · seq 6 | nothing leaves Withdrawn | InadmissibleTransition · `0xFF` |
| 08 | `vector-08-reserved-discriminant-refused` | Granted · seq 0 | state byte 0x04 | ReservedDiscriminant · `0x07` |
| 09 | `vector-09-reserved-flag-bit-refused` | Granted · seq 0 | flag bit 7 | ReservedFlagBit · `0x07` |
| 10 | `vector-10-undersize-frame-refused` | Granted · seq 0 | 95 bytes | WireFormatLength · `0x07` |
| 11 | `vector-11-oversize-frame-refused` | Granted · seq 0 | 97 bytes | WireFormatLength · `0x07` |
| 12 | `vector-12-wrong-manifest-refused` | Granted · seq 0 | manifest 0x0008 | ManifestMismatch · `0xFF` |
| 13 | `vector-13-forged-signature-refused` | Granted · seq 0 | one bit of R flipped | SignatureInvalid · `0x08` |
| 14 | `vector-14-wrong-key-refused` | Granted · seq 0 | signed with the RFC 8032 TEST 2 key | SignatureInvalid · `0x08` |
| 15 | `vector-15-replayed-sequence-refused` | Granted · seq 5 | sequence 5 after 5 | Replay · `0x08` |
| 16 | `vector-16-terminal-flag-mismatch-refused` | Granted · seq 0 | Withdrawn without the terminal flag | TerminalFlagMismatch · `0x07` |
| 17 | `vector-17-bad-magic-refused` | Granted · seq 0 | AXC1 | BadMagic · `0x07` |
| 18 | `vector-18-reserved-field-nonzero-refused` | Granted · seq 0 | byte 31 set | ReservedFieldNonZero · `0x07` |
| 19 | `vector-19-non-canonical-signature-refused` | Granted · seq 0 |  | SignatureInvalid · `0x08` |
| 20 | `vector-20-guardian-on-single-party-refused` | Granted · seq 0 |  | SignatureInvalid · `0x08` |

## Integrity

`SHA256SUMS` covers every vector file. The generator is
[`examples/gen_vectors.rs`](../examples/gen_vectors.rs); CI checks the files
against it byte for byte and runs [`tests/vectors.rs`](../tests/vectors.rs),
which executes every vector.

```bash
sha256sum -c SHA256SUMS
cargo run --example gen_vectors --features std -- --check
```

The vectors are dedicated to the public domain under [CC0-1.0](./LICENSE); the
specification text remains CC-BY-SA-4.0.
