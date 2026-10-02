// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! The publication gate under load: a producer thread publishes as fast as it
//! can while the trusted path withdraws consent. Nothing is published after
//! the withdrawal returns.
//!
//! `cargo run --release --example publication_gate --features std`

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use axonos_consent::wire::{assemble, ConsentRecord};
use axonos_consent::{ConsentMachine, ConsentState, Ed25519Strict, Suppressed};
use ed25519_dalek::{Signer, SigningKey};

fn main() {
    let trusted_path = SigningKey::from_bytes(&[0x42; 32]);
    let machine = ConsentMachine::new(1, trusted_path.verifying_key().to_bytes(), Ed25519Strict)
        .expect("a valid key");
    let machine = Arc::new(Mutex::new(machine));
    let stop = Arc::new(AtomicBool::new(false));

    // The gate is shared by reference; transitions need the machine.
    let producer = {
        let machine = machine.clone();
        let stop = stop.clone();
        thread::spawn(move || {
            let mut committed = 0u64;
            while !stop.load(Ordering::Relaxed) {
                let refused = {
                    let m = machine.lock().expect("lock");
                    match m.gate().try_publish() {
                        Ok(_) => {
                            committed += 1;
                            None
                        }
                        Err(why) => Some(why),
                    }
                };
                if refused == Some(Suppressed::Withdrawn) {
                    break;
                }
            }
            committed
        })
    };

    thread::sleep(std::time::Duration::from_millis(20));
    let record = ConsentRecord::new(ConsentState::Withdrawn, 1, 1, 0);
    let frame = assemble(&record, &trusted_path.sign(&record.encode()).to_bytes());
    let at_withdrawal = {
        let mut m = machine.lock().expect("lock");
        m.handle(&frame).expect("a signed withdrawal");
        m.gate().published()
    };
    let committed = producer.join().expect("producer");
    stop.store(true, Ordering::Relaxed);
    let after = machine.lock().expect("lock").gate().published();
    println!("published before the withdrawal returned: {at_withdrawal}");
    println!("published in total:                       {after} ({committed} by the producer)");
    assert_eq!(
        after, at_withdrawal,
        "nothing is published after withdrawal"
    );
}
