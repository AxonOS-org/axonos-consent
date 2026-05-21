//! Kernel interlock: an `ObservationGate` is consulted on every IPC publication.
//!
//! The kernel's SPSC ring producer reads the gate before publishing each
//! `IntentObservation`. A `Suspended` or `Withdrawn` state suppresses publication.

use crate::state::{ConsentMachine, ConsentState};

/// The gate consulted by the kernel IPC publication path. A `ConsentMachine`
/// is a gate; in some integrations a thinner wrapper around just the state byte
/// may be used to avoid pulling the full machine into the publish hot path.
pub trait ObservationGate {
    /// Returns true if the publication should proceed; false if it should be
    /// suppressed and a typed error emitted to the consumer.
    fn should_publish(&self) -> bool;

    /// Returns the appropriate ABI error code for the current suppression state.
    /// Only meaningful if `should_publish()` returned `false`.
    fn suppression_code(&self) -> u8;
}

impl ObservationGate for ConsentMachine {
    fn should_publish(&self) -> bool {
        matches!(self.state(), ConsentState::Granted)
    }

    fn suppression_code(&self) -> u8 {
        match self.state() {
            ConsentState::Granted => 0,      // n/a; should_publish() was true
            ConsentState::Suspended => 0x05, // ConsentSuspended per Standard §7.4
            ConsentState::Withdrawn => 0x06, // ConsentWithdrawn per Standard §7.4
        }
    }
}
