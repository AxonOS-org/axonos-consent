//! The consent finite-state machine.
//!
//! Three states: `Granted`, `Suspended`, `Withdrawn`. `Withdrawn` is terminal.
//!
//! See [SPEC §2 and §3](https://github.com/AxonOS-org/axonos-consent/blob/main/SPEC.md)
//! for the normative semantics.

use core::sync::atomic::{AtomicU8, Ordering};

use crate::error::ConsentError;
use crate::wire::ConsentEvent;

/// One of the three consent states.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum ConsentState {
    /// Observations flow normally.
    Granted = 0x01,
    /// Observations do not flow; consumers receive a typed backpressure error.
    /// Resumable to `Granted` through the trusted path.
    Suspended = 0x02,
    /// Streams terminated. Terminal — requires fresh manifest install.
    Withdrawn = 0x03,
}

impl ConsentState {
    /// Decode from the on-wire discriminant byte.
    pub fn from_u8(b: u8) -> Result<Self, ConsentError> {
        match b {
            0x01 => Ok(Self::Granted),
            0x02 => Ok(Self::Suspended),
            0x03 => Ok(Self::Withdrawn),
            _ => Err(ConsentError::ReservedDiscriminant),
        }
    }

    /// True if this state is terminal (i.e., `Withdrawn`).
    pub fn is_terminal(&self) -> bool {
        matches!(self, ConsentState::Withdrawn)
    }
}

/// The consent state machine for one manifest installation.
pub struct ConsentMachine {
    manifest_id: u16,
    trusted_path_pubkey: [u8; 32],
    state: AtomicU8,
}

impl ConsentMachine {
    /// Create a new machine for a freshly installed manifest. Initial state is
    /// `Granted` per SPEC §2.2.
    pub fn new(manifest_id: u16, trusted_path_pubkey: [u8; 32]) -> Self {
        Self {
            manifest_id,
            trusted_path_pubkey,
            state: AtomicU8::new(ConsentState::Granted as u8),
        }
    }

    /// Current state. Reads with acquire ordering so the IPC publication path
    /// sees the latest state from the writer.
    pub fn state(&self) -> ConsentState {
        // The invariant maintained by handle_event() is that the byte stored in
        // the atomic is always one of the three discriminants. A value outside
        // that set indicates a memory-corruption event already; we trap with
        // unreachable!() which is implemented as a panic.
        match self.state.load(Ordering::Acquire) {
            0x01 => ConsentState::Granted,
            0x02 => ConsentState::Suspended,
            0x03 => ConsentState::Withdrawn,
            other => unreachable!("invariant violated: state byte = 0x{:02X}", other),
        }
    }

    /// Process one trusted-path event. Returns the resulting state on success.
    ///
    /// SPEC §4.1: the bound on this function is ≤ 1648 cycles, L1-proven by Kani.
    pub fn handle_event(&mut self, event: ConsentEvent) -> Result<ConsentState, ConsentError> {
        if event.manifest_id != self.manifest_id {
            return Err(ConsentError::ManifestMismatch);
        }

        crate::crypto::verify_truncated(&event, &self.trusted_path_pubkey)?;

        let target = ConsentState::from_u8(event.state)?;
        let current = self.state();

        if !is_admissible_transition(current, target) {
            return Err(ConsentError::InadmissibleTransition);
        }

        self.state.store(target as u8, Ordering::SeqCst);
        Ok(target)
    }

    /// The manifest ID this machine is bound to.
    pub fn manifest_id(&self) -> u16 {
        self.manifest_id
    }
}

/// Is the transition `from → to` admissible per SPEC §3.1?
///
/// Made `pub` so test code and the kernel interlock can query the transition
/// graph without instantiating a machine.
///
/// Admissible transitions: identity (idempotent re-application), pause/resume,
/// and revocation from either active state. The two inadmissible transitions
/// (`Withdrawn → Granted` and `Withdrawn → Suspended`) are not in the match
/// arm, so this function returns false for them.
pub const fn is_admissible_transition(from: ConsentState, to: ConsentState) -> bool {
    use ConsentState::{Granted, Suspended, Withdrawn};
    matches!(
        (from, to),
        (Granted, Granted)
            | (Suspended, Suspended)
            | (Withdrawn, Withdrawn)
            | (Granted, Suspended)
            | (Suspended, Granted)
            | (Granted, Withdrawn)
            | (Suspended, Withdrawn)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admissible_transitions_per_spec() {
        use ConsentState::{Granted, Suspended, Withdrawn};
        for (from, to) in [
            (Granted, Granted),
            (Suspended, Suspended),
            (Withdrawn, Withdrawn),
            (Granted, Suspended),
            (Suspended, Granted),
            (Granted, Withdrawn),
            (Suspended, Withdrawn),
        ] {
            assert!(
                is_admissible_transition(from, to),
                "expected admissible: {:?} → {:?}",
                from,
                to
            );
        }
        assert!(!is_admissible_transition(Withdrawn, Granted));
        assert!(!is_admissible_transition(Withdrawn, Suspended));
    }

    #[test]
    fn withdrawn_is_terminal() {
        assert!(ConsentState::Withdrawn.is_terminal());
        assert!(!ConsentState::Granted.is_terminal());
        assert!(!ConsentState::Suspended.is_terminal());
    }

    #[test]
    fn from_u8_round_trips_known_states() {
        for s in [
            ConsentState::Granted,
            ConsentState::Suspended,
            ConsentState::Withdrawn,
        ] {
            assert_eq!(ConsentState::from_u8(s as u8).unwrap(), s);
        }
    }

    #[test]
    fn from_u8_rejects_unknown() {
        for b in [0x00u8, 0x04, 0x10, 0x7F, 0xFF] {
            assert!(matches!(
                ConsentState::from_u8(b),
                Err(ConsentError::ReservedDiscriminant)
            ));
        }
    }
}
