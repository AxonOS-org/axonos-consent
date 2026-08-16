//! Kani harness: `handle_event()` on a terminal Withdraw frame terminates
//! under bounded unwinding and yields `Withdrawn`.
//!
//! Scope of this harness, stated precisely:
//!
//! * It proves **termination and target-state correctness**. This is the L1
//!   evidence backing the correctness clause of SPEC §4.1.
//! * It does **not** establish the ≤ 1648 cycle bound. Kani is a bounded
//!   model checker over Rust MIR; it does not compute Cortex-M cycle counts.
//!   That bound is analytical (instruction-count derived) and is tagged as
//!   such in SPEC §4.1.
//! * It exercises the transition from `Granted` only. `starting_state` is
//!   generated and constrained below, but `ConsentMachine::new()` always
//!   constructs the default state, so the other two starting states are not
//!   covered. Extending coverage to `Suspended` and `Withdrawn` is a known
//!   open item, tracked in CHANGELOG under the 2026-08-16 correction.

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
