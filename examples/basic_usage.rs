// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! One installation, from boot to withdrawal.
//!
//! `cargo run --example basic_usage --features std`

use axonos_consent::wire::{assemble, ConsentRecord};
use axonos_consent::{ConsentError, ConsentMachine, ConsentState, Ed25519Strict};
use ed25519_dalek::{Signer, SigningKey};

fn main() -> Result<(), ConsentError> {
    // The trusted path holds the signing key. The kernel holds only the
    // public half, and that is all it ever needs.
    let trusted_path = SigningKey::from_bytes(&[0x42; 32]);
    let sign =
        |record: ConsentRecord| assemble(&record, &trusted_path.sign(&record.encode()).to_bytes());
    let manifest = 7;
    let mut machine = ConsentMachine::new(
        manifest,
        trusted_path.verifying_key().to_bytes(),
        Ed25519Strict,
    )?;
    println!("installed         {:?}", machine.state());

    // The publication path commits through the gate.
    println!("publish           {:?}", machine.gate().try_publish());

    machine.handle(&sign(ConsentRecord::new(
        ConsentState::Suspended,
        manifest,
        1,
        1_000,
    )))?;
    println!("pause             {:?}", machine.state());
    println!("publish           {:?}", machine.gate().try_publish());

    machine.handle(&sign(ConsentRecord::new(
        ConsentState::Granted,
        manifest,
        2,
        2_000,
    )))?;
    println!("resume            {:?}", machine.state());

    let withdrawal = sign(ConsentRecord::new(
        ConsentState::Withdrawn,
        manifest,
        3,
        3_000,
    ));
    machine.handle(&withdrawal)?;
    println!("withdraw          {:?}", machine.state());
    println!("publish           {:?}", machine.gate().try_publish());

    // What the machine refuses.
    println!("replay            {:?}", machine.handle(&withdrawal));
    let mut forged = sign(ConsentRecord::new(
        ConsentState::Granted,
        manifest,
        4,
        4_000,
    ));
    forged[40] ^= 1;
    println!("forged signature  {:?}", machine.handle(&forged));
    println!(
        "leave Withdrawn   {:?}",
        machine.handle(&sign(ConsentRecord::new(
            ConsentState::Granted,
            manifest,
            5,
            5_000
        )))
    );
    println!("persist           {:?}", machine.persisted());
    Ok(())
}
