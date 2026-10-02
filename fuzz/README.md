# Fuzzing

Three [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) targets search for
inputs that would break the specification. They complement the Kani proofs in
[`src/proofs.rs`](../src/proofs.rs), which cover bounded inputs exhaustively.

| Target | Drives | Fails on |
|:--|:--|:--|
| `frame_decode` | `Frame::parse`, `ConsentRecord::decode` (SPEC §6) | a panic, or an accepted record that does not re-encode to its own bytes |
| `fsm_sequence` | `ConsentMachine::handle` over streams of records, signatures stubbed out (SPEC §2, §3, §7.5, §9) | a publication while not `Granted`, a sequence that moves backwards, anything leaving `Withdrawn` |
| `auth_forgery` | `ConsentMachine::handle` under `Ed25519Strict` and the RFC 8032 TEST 1 key (SPEC §7) | any admitted frame: the seed corpus holds only refused frames, so an admission is a forgery |

```bash
cargo +nightly fuzz run auth_forgery -- -max_total_time=600
```

CI runs each target for 60 seconds on every change. That is a regression net;
a real campaign runs for hours, and a crash it finds becomes a test.

<sub>© 2026 Denis Yermakou · The AxonOS Project · connect@axonos.org</sub>
