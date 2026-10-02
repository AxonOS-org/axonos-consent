// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! The gate on real threads and real hardware memory ordering. The exhaustive
//! check is the loom model in `src/gate.rs`; this is its counterpart at speed.

#![cfg(feature = "ed25519")]

mod common;
use common::*;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;

use axonos_consent::wire::ConsentRecord;
use axonos_consent::{ConsentMachine, ConsentState, Ed25519Strict, Suppressed};

#[test]
fn nothing_is_published_after_a_withdrawal_returns() {
    for round in 0..200 {
        let machine = Arc::new(std::sync::RwLock::new(
            ConsentMachine::new(MANIFEST, public(&patient()), Ed25519Strict).expect("key"),
        ));
        let slots: Arc<Vec<AtomicU32>> = Arc::new((0..1024).map(|_| AtomicU32::new(0)).collect());
        let go = Arc::new(AtomicBool::new(false));
        let producer = {
            let (machine, slots, go) = (machine.clone(), slots.clone(), go.clone());
            thread::spawn(move || {
                while !go.load(Ordering::Acquire) {}
                loop {
                    let m = machine.read().expect("read");
                    let next = m.gate().published() as usize % slots.len();
                    slots[next].store(round + 1, Ordering::Relaxed);
                    match m.gate().try_publish() {
                        Ok(_) => {}
                        Err(Suppressed::Withdrawn) => return,
                        Err(other) => panic!("{other:?}"),
                    }
                }
            })
        };
        let frame = sign(
            &patient(),
            ConsentRecord::new(ConsentState::Withdrawn, MANIFEST, 1, 0),
        );
        go.store(true, Ordering::Release);
        let at_withdrawal = {
            let mut m = machine.write().expect("write");
            m.handle(&frame).expect("withdraw");
            m.gate().published()
        };
        producer.join().expect("producer");
        assert_eq!(
            machine.read().expect("read").gate().published(),
            at_withdrawal
        );
    }
}
