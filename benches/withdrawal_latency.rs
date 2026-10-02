// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! Host-only timings: `cargo bench --features std`.
//!
//! These numbers describe the machine you run them on, not a Cortex-M target,
//! and are not evidence for any bound in the specification. They exist to
//! catch regressions and to show where the time goes: almost all of it in
//! Ed25519 verification, almost none in the state machine.

use std::hint::black_box;
use std::time::Instant;

use axonos_consent::wire::{assemble, ConsentRecord};
use axonos_consent::{ConsentMachine, ConsentState, Ed25519Strict, PublicationGate};
use ed25519_dalek::{Signer, SigningKey};

const N: u64 = 2_000;

fn main() {
    let key = SigningKey::from_bytes(&[0x42; 32]);
    let frames: Vec<_> = (1..=N)
        .map(|seq| {
            let state = if seq % 2 == 1 {
                ConsentState::Suspended
            } else {
                ConsentState::Granted
            };
            let record = ConsentRecord::new(state, 1, seq, seq);
            assemble(&record, &key.sign(&record.encode()).to_bytes())
        })
        .collect();
    let mut machine =
        ConsentMachine::new(1, key.verifying_key().to_bytes(), Ed25519Strict).expect("key");
    let start = Instant::now();
    for frame in &frames {
        black_box(machine.handle(black_box(frame)).expect("admitted"));
    }
    let per_frame = start.elapsed().as_nanos() / u128::from(N);

    let gate = PublicationGate::new();
    let start = Instant::now();
    for _ in 0..N * 1_000 {
        black_box(gate.try_publish().ok());
    }
    let per_publish = start.elapsed().as_nanos() as f64 / (N * 1_000) as f64;

    println!("host · handle() with Ed25519 verification: {per_frame} ns per frame");
    println!("host · try_publish():                      {per_publish:.1} ns per commit");
}
