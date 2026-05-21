//! Kani harness: no sequence of wire-format inputs can drive the FSM through
//! a non-admissible transition. Specifically: from Withdrawn, no input can
//! reach Granted or Suspended.

use axonos_consent::{ConsentMachine, ConsentState};
use axonos_consent::state::is_admissible_transition;

#[kani::proof]
fn fsm_no_invalid_transitions() {
    use ConsentState::*;

    // Two non-admissible transitions per SPEC §3.2:
    assert!(!is_admissible_transition(Withdrawn, Granted));
    assert!(!is_admissible_transition(Withdrawn, Suspended));

    // All seven admissible transitions:
    assert!(is_admissible_transition(Granted, Granted));
    assert!(is_admissible_transition(Suspended, Suspended));
    assert!(is_admissible_transition(Withdrawn, Withdrawn));
    assert!(is_admissible_transition(Granted, Suspended));
    assert!(is_admissible_transition(Suspended, Granted));
    assert!(is_admissible_transition(Granted, Withdrawn));
    assert!(is_admissible_transition(Suspended, Withdrawn));
}
