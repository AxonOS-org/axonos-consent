#![no_main]
//! Fuzz target — the consent finite-state machine under arbitrary event streams.
//!
//! The fuzz input is read as `[ 32-byte key | 2-byte manifest id | sequence of
//! target-state bytes ]`. Each sequence byte becomes a *correctly signed*
//! `ConsentEvent`, so the signature check passes and what is exercised is the
//! FSM transition logic itself — not the crypto path.
//!
//! Invariants asserted after every event:
//!   1. `handle_event` never panics — reaching the next line proves it.
//!   2. `state()` never panics — the stored byte is always a valid state.
//!   3. `Withdrawn` is terminal (SPEC §3.3): once entered, never left.
//!   4. Every *accepted* transition is admissible (SPEC §3.1).

use libfuzzer_sys::fuzz_target;
use axonos_consent::crypto::compute_tag;
use axonos_consent::state::is_admissible_transition;
use axonos_consent::{ConsentEvent, ConsentMachine, ConsentState};

fuzz_target!(|data: &[u8]| {
    // Need key (32) + manifest id (2) + at least one event byte.
    if data.len() < 35 {
        return;
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(&data[..32]);
    let manifest_id = u16::from_le_bytes([data[32], data[33]]);
    let sequence = &data[34..];

    let mut machine = ConsentMachine::new(manifest_id, key);
    assert_eq!(
        machine.state(),
        ConsentState::Granted,
        "initial state must be Granted (SPEC §2.2)",
    );

    let mut entered_withdrawn = false;

    for (i, &raw) in sequence.iter().enumerate() {
        // Map the fuzz byte to a discriminant in 0..=3. 0 is the reserved
        // (invalid) discriminant — included deliberately so the decoder path
        // inside handle_event is exercised alongside the valid transitions.
        let state_byte = raw % 4;

        let mut event = ConsentEvent {
            state: state_byte,
            flags: 0,
            manifest_id,
            timestamp_us: i as u64,
            sig_truncated: 0,
        };
        // Sign correctly — the FSM is the subject here, not the signature path.
        event.sig_truncated = compute_tag(&event, &key);

        let before = machine.state();             // invariant 2
        let result = machine.handle_event(event); // invariant 1
        let after = machine.state();              // invariant 2

        // Invariant 3 — terminality of Withdrawn.
        if before == ConsentState::Withdrawn {
            assert_eq!(
                after, ConsentState::Withdrawn,
                "terminal violated: left the Withdrawn state",
            );
        }
        if after == ConsentState::Withdrawn {
            entered_withdrawn = true;
        }
        if entered_withdrawn {
            assert_eq!(
                after, ConsentState::Withdrawn,
                "terminal violated: re-entered an active state after Withdrawn",
            );
        }

        // Invariant 4 — an accepted transition is always admissible.
        if let Ok(reached) = result {
            assert_eq!(after, reached, "state() disagrees with handle_event() result");
            assert!(
                is_admissible_transition(before, reached),
                "inadmissible transition was accepted: {before:?} -> {reached:?}",
            );
        }
    }
});
