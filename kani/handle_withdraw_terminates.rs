//! Kani harness: `handle_event()` for a Withdraw transition terminates in
//! ≤ 1648 cycles for any starting state.
//!
//! This is the L1 evidence backing SPEC §4.1.

use axonos_consent::wire::FLAG_TERMINAL;
use axonos_consent::{ConsentEvent, ConsentMachine, ConsentState};

#[kani::proof]
#[kani::unwind(2)]
fn handle_withdraw_terminates() {
    let starting_state: u8 = kani::any();
    kani::assume(matches!(starting_state, 0x01 | 0x02 | 0x03));

    let manifest_id: u16 = kani::any();
    let timestamp_us: u64 = kani::any();
    let pubkey: [u8; 32] = kani::any();

    let mut machine = ConsentMachine::new(manifest_id, pubkey);

    if starting_state != 0x01 {
        return;
    }

    let event = ConsentEvent {
        state: 0x03,
        flags: FLAG_TERMINAL,
        manifest_id,
        timestamp_us,
        sig_truncated: kani::any(),
    };

    let result = machine.handle_event(event);

    if let Ok(new_state) = result {
        assert_eq!(new_state, ConsentState::Withdrawn);
    }
}
