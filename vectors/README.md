# Conformance test vectors

These are the canonical wire-format vectors that any conformant implementation
of the **AxonOS Consent Specification v0.3.0** must accept or refuse as documented.

## License

The vectors in this directory are dedicated to the public domain under
[Creative Commons Zero v1.0 (CC0-1.0)](./LICENSE). Use them in your own
implementation, your test suite, your textbook, or anywhere else, without
attribution.

## Vector format

Each vector is a single binary file with a sibling `.expected.json`:

```
vector-NN-description.bin       16-byte wire-format input
vector-NN-description.expected.json   { "verdict": "accept"|"refuse", "reason": "...", "post_state": "Granted"|"Suspended"|"Withdrawn"|null }
```

## Vector index (v0.3.0)

| # | File | What it tests |
|:---:|:---|:---|
| 01 | granted-to-suspended | A valid Granted → Suspended transition |
| 02 | suspended-to-granted | A valid resume |
| 03 | granted-to-withdrawn | A valid revocation from active state |
| 04 | suspended-to-withdrawn | A valid revocation from paused state |
| 05 | idempotent-granted | Idempotent re-application of Granted |
| 06 | withdrawn-to-granted-refused | Inadmissible: Withdrawn → Granted |
| 07 | withdrawn-to-suspended-refused | Inadmissible: Withdrawn → Suspended |
| 08 | reserved-discriminant-refused | State byte = 0x00 (reserved) |
| 09 | reserved-flag-bit-refused | Flag bit 7 set |
| 10 | undersize-buffer-refused | 15 bytes (one short) |
| 11 | oversize-buffer-refused | 17 bytes (one long) |
| 12 | wrong-manifest-id-refused | manifest_id mismatch |

## How to use these vectors

Treat each vector as a black-box test of your implementation:

```
for each vector v:
    let result = your_implementation.decode_and_handle(v.input_bytes);
    assert result.verdict == v.expected.verdict;
    if v.expected.post_state is not null:
        assert result.post_state == v.expected.post_state;
```

A conformant implementation produces the documented verdict for each vector.

## Binary content

The vectors in `*.bin` are bit-exact; do not let your text editor "fix"
the line endings. They are 16 raw bytes each.
