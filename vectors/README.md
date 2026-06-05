# Conformance test vectors — AxonOS Consent wire format

These are the canonical conformance vectors for the **AxonOS Consent** kernel/SDK
boundary: the 16-byte wire record (Consent Specification §6) and the three-state
consent machine (§3). Any conformant implementation **MUST** decode and evaluate
each vector to the documented verdict.

The vectors are generated from, and byte-for-byte consistent with, the reference
implementation in this repository (`src/wire.rs`, `src/state.rs`, `src/error.rs`).
They are self-contained and carry no dependency on any external party.

## Licence

The vectors in this directory are dedicated to the public domain under
[Creative Commons Zero v1.0 (CC0-1.0)](./LICENSE). Use them in your own
implementation, test suite, or textbook, without attribution.

## The wire record (16 bytes, little-endian)

```
┌─ Offset ─┬─ Size ─┬─ Field ────────┬─ Type ──────────────────────┐
│   0      │   1    │ state          │ u8  (0x01 Granted, 0x02      │
│          │        │                │      Suspended, 0x03 Withdrawn)│
│   1      │   1    │ flags          │ u8  (bitfield, see below)    │
│   2      │   2    │ manifest_id    │ u16 little-endian            │
│   4      │   8    │ timestamp_us   │ u64 little-endian (monotonic)│
│  12      │   4    │ sig_truncated  │ u32 little-endian (integrity)│
└──────────┴────────┴────────────────┴──────────────────────────────┘
```

**Flags** — defined mask `0x0F`. Bit 0 `terminal`, bit 1 `from-secure-world`,
bit 2 `replay-tolerant`, bit 3 `guardian`. **Any bit outside `0x0F` (bits 4–7)
set MUST be refused** with `ReservedFlagBit`.

A decoder **MUST** refuse any input that is not exactly 16 bytes
(`WireFormatLength`) and any `state` outside `{0x01, 0x02, 0x03}`
(`ReservedDiscriminant`).

## Decode order (normative)

The reference decoder checks in this order; vectors are constructed to exercise
each branch in isolation:

1. length ≠ 16 → `WireFormatLength`;
2. `state` not in `{1,2,3}` → `ReservedDiscriminant`;
3. reserved flag bit set → `ReservedFlagBit`;
4. (transition layer) event `manifest_id` ≠ context manifest → `ManifestMismatch`;
5. (transition layer) `(from_state → state)` not admissible → `InadmissibleTransition`.

Admissible transitions: `(G→G) (S→S) (W→W) (G→S) (S→G) (G→W) (S→W)`. `Withdrawn`
is terminal: `W→G` and `W→S` are refused.

## File format

Each vector is a pair:

```
vector-NN-description.bin            the raw wire buffer (16 bytes, except the two length vectors: 15 and 17)
vector-NN-description.expected.json  the expected result
```

`.expected.json` schema:

```json
{
  "description": "human-readable summary",
  "layer": "decode" | "transition",
  "from_state": "Granted" | "Suspended" | "Withdrawn" | null,
  "context_manifest_id": 1 | null,
  "input_len": 16,
  "verdict": "accept" | "refuse",
  "error": "ReservedDiscriminant" | null,
  "error_message": "reserved state discriminant" | null,
  "post_state": "Suspended" | null
}
```

`layer: "decode"` vectors fail (or would pass) at `from_bytes` alone and need no
state context; `layer: "transition"` vectors are applied to a consent machine
initialised in `from_state` for `context_manifest_id`. On `accept`, `post_state`
is the resulting state; on `refuse`, `error` is the reference `ConsentError`
variant and `error_message` its display string.

**Canonical context.** Transition vectors assume a machine initialised for
`manifest_id = 0x0001`. The `timestamp_us` and `sig_truncated` fields carry fixed
canonical values; neither participates in the decode or admissibility decision
(the full Ed25519 signature is verified out-of-band per §7).

## Vector index

| # | File | Layer | From → event | Verdict | Error / post-state |
|:---:|:---|:---|:---|:---:|:---|
| 01 | granted-to-suspended | transition | Granted → Suspended | accept | post: Suspended |
| 02 | suspended-to-granted | transition | Suspended → Granted | accept | post: Granted |
| 03 | granted-to-withdrawn | transition | Granted → Withdrawn | accept | post: Withdrawn |
| 04 | suspended-to-withdrawn | transition | Suspended → Withdrawn | accept | post: Withdrawn |
| 05 | idempotent-granted | transition | Granted → Granted | accept | post: Granted |
| 06 | withdrawn-to-granted-refused | transition | Withdrawn → Granted | refuse | InadmissibleTransition |
| 07 | withdrawn-to-suspended-refused | transition | Withdrawn → Suspended | refuse | InadmissibleTransition |
| 08 | reserved-discriminant-refused | decode | state 0x00 | refuse | ReservedDiscriminant |
| 09 | reserved-flag-bit-refused | decode | flag bit 7 set | refuse | ReservedFlagBit |
| 10 | undersize-buffer-refused | decode | 15-byte input | refuse | WireFormatLength |
| 11 | oversize-buffer-refused | decode | 17-byte input | refuse | WireFormatLength |
| 12 | wrong-manifest-id-refused | transition | manifest 0x0002 ≠ 0x0001 | refuse | ManifestMismatch |

## How to use these vectors

```text
for each vector:
    buf      = read(vector.bin)
    expected = read(vector.expected.json)
    if expected.layer == "decode":
        result = your_impl.decode(buf)            # ConsentEvent::from_bytes equivalent
    else:
        machine = ConsentMachine(expected.context_manifest_id, state = expected.from_state)
        result  = machine.handle(buf)             # decode + manifest check + transition
    assert result.verdict      == expected.verdict
    assert result.error        == expected.error          # on refuse
    assert result.post_state   == expected.post_state     # on accept
```

## Integrity

`SHA256SUMS` lists the SHA-256 of every `.bin` and `.expected.json` in this
directory. Verify with:

```bash
cd vectors && sha256sum -c SHA256SUMS
```
