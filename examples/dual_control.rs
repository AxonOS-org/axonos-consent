// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! A patient and a guardian: either can stop the flow, only both can resume it.
//!
//! `cargo run --example dual_control --features std`

use axonos_consent::wire::{assemble, ConsentRecord};
use axonos_consent::{ConsentError, ConsentState, DualControlMachine, Ed25519Strict};
use ed25519_dalek::{Signer, SigningKey};

fn main() -> Result<(), ConsentError> {
    let patient = SigningKey::from_bytes(&[0x11; 32]);
    let guardian = SigningKey::from_bytes(&[0x22; 32]);
    let sign = |key: &SigningKey, record: ConsentRecord| {
        assemble(&record, &key.sign(&record.encode()).to_bytes())
    };
    let manifest = 7;
    let mut machine = DualControlMachine::new(
        manifest,
        patient.verifying_key().to_bytes(),
        guardian.verifying_key().to_bytes(),
        Ed25519Strict,
    )?;

    // The kernel's monotonic clock, in microseconds.
    let mut now = 1_000_000;

    let pause = ConsentRecord::new(ConsentState::Suspended, manifest, 1, 0);
    println!(
        "guardian pauses   {:?}",
        machine.propose(&sign(&guardian, pause.by_guardian()), now)?
    );

    now += 5_000_000;
    let resume = ConsentRecord::new(ConsentState::Granted, manifest, 1, 0);
    println!(
        "patient resumes   {:?}",
        machine.propose(&sign(&patient, resume), now)?
    );

    now += 30_000_000;
    let agree = ConsentRecord::new(ConsentState::Granted, manifest, 2, 0).by_guardian();
    println!(
        "guardian agrees   {:?}",
        machine.propose(&sign(&guardian, agree), now)?
    );

    now += 1_000;
    let stop = ConsentRecord::new(ConsentState::Withdrawn, manifest, 2, 0);
    println!(
        "patient withdraws {:?}",
        machine.propose(&sign(&patient, stop), now)?
    );
    println!("persist           {:?}", machine.persisted());
    Ok(())
}
