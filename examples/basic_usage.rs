//! Basic usage of the consent state machine.
//!
//! Run with: `cargo run --example basic_usage --features std`

use axonos_consent::{
    ConsentEvent, ConsentMachine, ConsentState,
    crypto::compute_tag,
    wire::{FLAG_TERMINAL, FLAG_REPLAY_TOLERANT},
    interlock::ObservationGate,
};

fn main() {
    println!("=== AxonOS Consent — basic usage example ===\n");

    // 1. Set up a machine for manifest #42 with a trusted-path public key.
    let trusted_path_pubkey = [0xC0u8; 32];
    let mut machine = ConsentMachine::new(42, trusted_path_pubkey);
    println!("Fresh machine: state = {:?}", machine.state());
    println!("  IPC publishes? {}\n", machine.should_publish());

    // 2. User suspends consent via the trusted path.
    let suspend_event = build_event(0x02, 42, &trusted_path_pubkey, 0);
    let new_state = machine.handle_event(suspend_event).unwrap();
    println!("After Suspend: state = {:?}", new_state);
    println!("  IPC publishes? {}", machine.should_publish());
    println!("  Suppression code: 0x{:02X}\n", machine.suppression_code());

    // 3. User resumes consent.
    let resume_event = build_event(0x01, 42, &trusted_path_pubkey, FLAG_REPLAY_TOLERANT);
    machine.handle_event(resume_event).unwrap();
    println!("After Resume: state = {:?}", machine.state());
    println!("  IPC publishes? {}\n", machine.should_publish());

    // 4. User withdraws consent — terminal.
    let withdraw_event = build_event(0x03, 42, &trusted_path_pubkey, FLAG_TERMINAL);
    machine.handle_event(withdraw_event).unwrap();
    println!("After Withdraw: state = {:?}", machine.state());
    println!("  IPC publishes? {}", machine.should_publish());
    println!("  Suppression code: 0x{:02X}\n", machine.suppression_code());

    // 5. Attempt to restore — refused.
    let restore_attempt = build_event(0x01, 42, &trusted_path_pubkey, 0);
    let outcome = machine.handle_event(restore_attempt);
    println!("Attempt to restore from Withdrawn: {:?}", outcome);
    println!("  State unchanged: {:?}", machine.state());
}

fn build_event(state: u8, mid: u16, pk: &[u8; 32], flags: u8) -> ConsentEvent {
    let mut e = ConsentEvent {
        state, flags, manifest_id: mid,
        timestamp_us: 1_700_000_000_000_000,
        sig_truncated: 0,
    };
    e.sig_truncated = compute_tag(&e, pk);
    e
}
