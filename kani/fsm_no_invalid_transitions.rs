//! Kani harness: the transition-admissibility table has the shape SPEC §3.2
//! requires — in particular, nothing is admissible out of `Withdrawn`.
//!
//! Scope: this checks the pure predicate `is_admissible_transition` over the
//! full 3×3 state space. It does **not** exercise wire-format inputs or the
//! FSM itself, so it is not a reachability proof over decoded events;
//! `handle_withdraw_terminates` covers the event path, from `Granted` only.

use axonos_consent::ConsentState;
use axonos_consent::state::is_admissible_transition;

#[kani::proof]
fn fsm_no_invalid_transitions() {
    use ConsentState::{Granted, Suspended, Withdrawn};

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
