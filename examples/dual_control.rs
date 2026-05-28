//! Multi-party (guardian) co-authorisation example.
//!
//! Models the clinical deployment from the AxonOS roadmap: a patient and a
//! guardian jointly control a manifest. Either may stop neural-data flow
//! alone; resuming it needs both.
//!
//! Run with: `cargo run --example dual_control --features std`

use axonos_consent::crypto::compute_tag;
use axonos_consent::dual_control::{DualControlMachine, Party};
use axonos_consent::ConsentEvent;

const MANIFEST_ID: u16 = 42;

/// Build a correctly-tagged consent event signed by `key`.
fn signed(state: u8, ts_us: u64, key: &[u8; 32]) -> ConsentEvent {
    let mut e = ConsentEvent {
        state,
        flags: 0,
        manifest_id: MANIFEST_ID,
        timestamp_us: ts_us,
        sig_truncated: 0,
    };
    e.sig_truncated = compute_tag(&e, key);
    e
}

fn main() {
    println!("=== AxonOS Consent — multi-party (guardian) co-authorisation ===\n");

    let patient_key = [0x11u8; 32];
    let guardian_key = [0x22u8; 32];
    let mut m = DualControlMachine::new(MANIFEST_ID, patient_key, guardian_key);
    println!("Fresh dual-control machine: state = {:?}\n", m.state());

    // 1. The patient suspends the flow unilaterally — no guardian needed.
    let r = m
        .propose(signed(0x02, 1_000, &patient_key), Party::Patient)
        .unwrap();
    println!("Patient suspends (unilateral): {:?}", r);
    println!("  state = {:?}\n", m.state());

    // 2. The patient wants to resume. Alone, this only arms a pending request.
    let r = m
        .propose(signed(0x01, 2_000, &patient_key), Party::Patient)
        .unwrap();
    println!("Patient requests resume: {:?}", r);
    println!("  pending authorisation by: {:?}", m.pending());
    println!("  state still = {:?}\n", m.state());

    // 3. The guardian co-authorises within the window — now it commits.
    let r = m
        .propose(signed(0x01, 3_000, &guardian_key), Party::Guardian)
        .unwrap();
    println!("Guardian co-authorises resume: {:?}", r);
    println!("  state = {:?}\n", m.state());

    // 4. Either party can withdraw unilaterally — terminal.
    let r = m
        .propose(signed(0x03, 4_000, &guardian_key), Party::Guardian)
        .unwrap();
    println!("Guardian withdraws (unilateral): {:?}", r);
    println!("  state = {:?}", m.state());
    println!("  terminal? {}\n", m.state().is_terminal());

    // 5. Neither party — nor both together — can resurrect a withdrawn manifest.
    let attempt = m.propose(signed(0x01, 5_000, &patient_key), Party::Patient);
    println!("Patient attempts to restore after withdrawal: {:?}", attempt);
    println!("\nThe safe direction is always one signature away;");
    println!("resuming the flow always takes two.");
}
