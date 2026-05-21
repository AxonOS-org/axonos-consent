//! The consent finite-state machine.
//!
//! Three states: `Granted`, `Suspended`, `Withdrawn`. `Withdrawn` is terminal.
//!
//! See [SPEC §2 and §3](https://github.com/AxonOS-org/axonos-standard/blob/main/SPEC.md)
//! for the normative semantics.

use crate::error::ConsentError;
use crate::wire::ConsentEvent;
use core::sync::atomic::{AtomicU8, Ordering};

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
    /// Create a new machine for a freshly installed manifest. Initial state is `Granted`
    /// per SPEC §2.2.
    pub fn new(manifest_id: u16, trusted_path_pubkey: [u8; 32]) -> Self {
        Self {
            manifest_id,
            trusted_path_pubkey,
            state: AtomicU8::new(ConsentState::Granted as u8),
        }
    }

    /// Current state. Reads with acquire ordering so the IPC publication path sees
    /// the latest state from the writer.
    pub fn state(&self) -> ConsentState {
        // SAFETY: state is only ever written with one of the three discriminants.
        match self.state.load(Ordering::Acquire) {
            0x01 => ConsentState::Granted,
            0x02 => ConsentState::Suspended,
            0x03 => ConsentState::Withdrawn,
            _ => unreachable!("invariant violated: state byte outside discriminant set"),
        }
    }

    /// Process one trusted-path event. Returns the resulting state on success.
    ///
    /// Returns:
    /// - `Ok(state)` if the transition is admissible (including idempotent re-application).
    /// - `Err(SignatureInvalid)` if the event's signature does not verify.
    /// - `Err(InadmissibleTransition)` if the requested transition is not admissible.
    /// - `Err(ManifestMismatch)` if the event targets a different manifest than this machine.
    /// - `Err(...)` for other validation failures per [`ConsentError`].
    ///
    /// SPEC §4.1: the bound on this function is ≤ 1648 cycles, L1-proven by Kani.
    pub fn handle_event(&mut self, event: ConsentEvent) -> Result<ConsentState, ConsentError> {
        // 1. Verify the manifest matches.
        if event.manifest_id != self.manifest_id {
            return Err(ConsentError::ManifestMismatch);
        }

        // 2. Verify the signature.
        crate::crypto::verify_truncated(&event, &self.trusted_path_pubkey)?;

        // 3. Decode the target state.
        let target = ConsentState::from_u8(event.state)?;

        // 4. Compute and validate the transition.
        let current = self.state();
        if !is_admissible_transition(current, target) {
            return Err(ConsentError::InadmissibleTransition);
        }

        // 5. Commit. SeqCst on write to ensure ordering with subsequent IPC publications.
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
/// This is `pub` so that test code and the kernel interlock can interrogate
/// the transition graph without instantiating a machine.
pub const fn is_admissible_transition(from: ConsentState, to: ConsentState) -> bool {
    use ConsentState::*;
    matches!(
        (from, to),
        // Identity (idempotent)
        (Granted,   Granted)
        | (Suspended, Suspended)
        | (Withdrawn, Withdrawn)
        // Pause / resume
        | (Granted,   Suspended)
        | (Suspended, Granted)
        // Revoke from either active state
        | (Granted,   Withdrawn)
        | (Suspended, Withdrawn)
    )
    // All other 9 - 7 = 2 transitions (Withdrawn → Granted, Withdrawn → Suspended)
    // are inadmissible.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admissible_transitions_per_spec() {
        use ConsentState::*;
        // The seven admissible transitions
        for (from, to) in [
            (Granted, Granted),
            (Suspended, Suspended),
            (Withdrawn, Withdrawn),
            (Granted, Suspended),
            (Suspended, Granted),
            (Granted, Withdrawn),
            (Suspended, Withdrawn),
        ] {
            assert!(is_admissible_transition(from, to), "expected admissible: {:?} → {:?}", from, to);
        }
        // The two inadmissible transitions
        assert!(!is_admissible_transition(Withdrawn, Granted));
        assert!(!is_admissible_transition(Withdrawn, Suspended));
    }

    #[test]
    fn withdrawn_is_terminal() {
        assert!(ConsentState::Withdrawn.is_terminal());
        assert!(!ConsentState::Granted.is_terminal());
        assert!(!ConsentState::Suspended.is_terminal());
    }
}
