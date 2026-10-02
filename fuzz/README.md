# Fuzzing

Three [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) targets search for
inputs that would break the specification. They complement the Kani proofs in
[`src/proofs.rs`](../src/proofs.rs), which cover bounded inputs exhaustively.

| Target | Drives | Fails on |
|:--|:--|:--|
| `frame_decode` | `Frame::parse`, `ConsentRecord::decode` (SPEC §6) | a panic, or an accepted record that does not re-encode to its own bytes |
| `fsm_sequence` | `ConsentMachine::handle` over streams of records, signatures stubbed out (SPEC §2, §3, §7.5, §9) | a publication while not `Granted`, a sequence that moves backwards, anything leaving `Withdrawn` |
| `auth_forgery` | `ConsentMachine::handle` under `Ed25519Strict` and a key whose secret no one holds (SPEC §7) | any admitted frame — under that key it can only be a forgery |

The key for `auth_forgery` is a curve point derived from SHA-512 of a fixed
label and multiplied by the cofactor; [`tools/check.py`](../tools/check.py)
repeats the derivation on every change. Nobody knows its discrete logarithm, so
no genuine signature under it exists for the fuzzer to rediscover from a seed.
Until 0.9.2 the target used the RFC 8032 TEST 1 key and seeded with a frame one
bit away from a genuine signature; libFuzzer flipped the bit back and reported
the genuine frame as a forgery. A forgery oracle must never be satisfiable by
anything except a forgery.

```bash
cargo +nightly fuzz run auth_forgery -- -max_total_time=600
```

CI runs each target for 60 seconds on every change. That is a regression net;
a real campaign runs for hours, and a crash it finds becomes a test.

<sub>© 2026 Denis Yermakou · The AxonOS Project · connect@axonos.org</sub>
