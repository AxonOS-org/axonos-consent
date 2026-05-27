# axonos-consent — fuzz suite

Coverage-guided fuzzing for the `axonos-consent` reference implementation,
built on [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) and libFuzzer.

The suite is **L2-class evidence**: a large, coverage-guided sample of the
unbounded input space. It complements the `kani/` harnesses, which are
**L1 evidence** — exhaustive proof over a bounded input space. Kani proves the
bounded core; fuzzing searches the rest. Both run in CI.

## Targets

| Target | Surface | Property searched for |
|:---|:---|:---|
| `wire_decode` | `ConsentEvent::from_bytes` | A panic or out-of-bounds read on any byte buffer (SPEC §6). |
| `roundtrip` | `from_bytes` ∘ `to_bytes` | A non-canonical encoding — two buffers denoting one event. |
| `fsm_sequence` | `ConsentMachine::handle_event` | An FSM-invariant violation under arbitrary signed-event streams (SPEC §2–§3). |

## Prerequisites

```sh
cargo install cargo-fuzz --locked
# cargo-fuzz requires a nightly toolchain:
rustup toolchain install nightly
```

## Build

```sh
cargo +nightly fuzz build
```

## Run

```sh
# Run one target until a crash is found or Ctrl-C:
cargo +nightly fuzz run wire_decode
cargo +nightly fuzz run roundtrip
cargo +nightly fuzz run fsm_sequence

# Time-boxed run (used by CI — 60 seconds per target):
cargo +nightly fuzz run wire_decode -- -max_total_time=60
```

## Triage

A crash is written to `fuzz/artifacts/<target>/`. Reproduce it with:

```sh
cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<crash-file>
```

A crash in any target is a specification or implementation defect — open a
security advisory rather than a public issue if it concerns `wire_decode`
or the signature path (see the repository `SECURITY.md`).

## Corpus

Seed inputs are committed under `corpus/<target>/`. libFuzzer grows the corpus
locally as it discovers new coverage; those discovered entries are left
untracked by `.gitignore`. Commit a discovered entry deliberately only when it
captures a coverage case worth keeping.
