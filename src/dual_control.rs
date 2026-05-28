//! Multi-party (guardian) co-authorisation for safety-increasing transitions.
//!
//! For clinical deployments — for example the ALS pilot described in the
//! canonical Standard's roadmap — a guardian co-authorises consent changes
//! together with the patient. This module implements the second-signature
//! path reserved in [DESIGN-RATIONALE §6.2](../docs/DESIGN-RATIONALE.md) and
//! specified in [SPEC §13](../SPEC.md).
//!
//! # The safe-direction principle
//!
//! Either authorised party may *unilaterally reduce* neural-data exposure:
//! moving to [`ConsentState::Suspended`] or [`ConsentState::Withdrawn`] never
//! requires agreement. Stopping the flow is always a single signature away —
//! the patient must never be unable to stop, and the guardian must never be
//! unable to stop on the patient's behalf.
//!
//! *Increasing* exposure — resuming the flow with `Suspended → Granted` —
//! requires **both** parties to authorise the same transition within a bounded
//! window. No sequence of signatures from a single party can resume the flow.
//! This is the property proven by the Kani harness
//! `co_authorisation_requires_two_parties`.
//!
//! # Example
//!
//! ```
//! use axonos_consent::dual_control::{CoAuthOutcome, DualControlMachine, Party};
//! use axonos_consent::ConsentState;
//! use axonos_consent::crypto::compute_tag;
//! use axonos_consent::ConsentEvent;
//!
//! let patient_key = [0x11u8; 32];
//! let guardian_key = [0x22u8; 32];
//! let mut m = DualControlMachine::new(7, patient_key, guardian_key);
//!
//! // Helper: build a correctly-tagged event for one party's key.
//! let signed = |state: u8, ts: u64, key: [u8; 32]| {
//!     let mut e = ConsentEvent { state, flags: 0, manifest_id: 7, timestamp_us: ts, sig_truncated: 0 };
//!     e.sig_truncated = compute_tag(&e, &key);
//!     e
//! };
//!
//! // Either party can suspend unilaterally.
//! let r = m.propose(signed(0x02, 10, patient_key), Party::Patient).unwrap();
//! assert_eq!(r, CoAuthOutcome::Applied(ConsentState::Suspended));
//!
//! // Resuming needs both parties. Patient alone only arms a pending request.
//! let r = m.propose(signed(0x01, 20, patient_key), Party::Patient).unwrap();
//! assert_eq!(r, CoAuthOutcome::PendingCoAuth(ConsentState::Suspended));
//!
//! // Guardian completes the co-authorisation within the window.
//! let r = m.propose(signed(0x01, 30, guardian_key), Party::Guardian).unwrap();
//! assert_eq!(r, CoAuthOutcome::Applied(ConsentState::Granted));
//! ```

use crate::crypto::verify_truncated;
use crate::error::ConsentError;
use crate::state::{is_admissible_transition, ConsentMachine, ConsentState};
use crate::wire::ConsentEvent;

/// Default co-authorisation window: two minutes, in microseconds.
///
/// A second authorisation that arrives more than this long after the first
/// does not commit; it instead re-arms a fresh pending request from the
/// arriving party.
pub const DEFAULT_CO_AUTH_WINDOW_US: u64 = 120_000_000;

/// An authorising party in a dual-control deployment.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Party {
    /// The patient — holder of the trusted-path (primary) key.
    Patient = 0x01,
    /// The co-authorising guardian — holder of the secondary key.
    Guardian = 0x02,
}

/// The outcome of a [`DualControlMachine::propose`] call.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CoAuthOutcome {
    /// The transition committed. The enclosed value is the new consent state.
    Applied(ConsentState),
    /// A safety-increasing transition was authorised by one party and now
    /// awaits a matching authorisation from the other party. The enclosed
    /// value is the *unchanged* current state.
    PendingCoAuth(ConsentState),
}

/// A half-completed safety-increasing authorisation.
#[derive(Copy, Clone)]
struct PendingGrant {
    proposer: Party,
    proposed_at_us: u64,
}

/// A consent machine that requires two distinct parties to *increase*
/// neural-data exposure, while letting either party *reduce* it unilaterally.
///
/// Wraps a single-party [`ConsentMachine`]; the inner machine holds the
/// patient (trusted-path) key, and the dual-control layer adds the guardian
/// key and the co-authorisation logic on top.
pub struct DualControlMachine {
    inner: ConsentMachine,
    guardian_pubkey: [u8; 32],
    pending: Option<PendingGrant>,
    window_us: u64,
}

impl DualControlMachine {
    /// Create a dual-control machine using [`DEFAULT_CO_AUTH_WINDOW_US`].
    ///
    /// The initial state is [`ConsentState::Granted`], consistent with the
    /// single-party machine: installing a signed manifest through the trusted
    /// path constitutes initial consent.
    pub fn new(manifest_id: u16, patient_pubkey: [u8; 32], guardian_pubkey: [u8; 32]) -> Self {
        Self::with_window(
            manifest_id,
            patient_pubkey,
            guardian_pubkey,
            DEFAULT_CO_AUTH_WINDOW_US,
        )
    }

    /// Create a dual-control machine with an explicit co-authorisation window
    /// (in microseconds).
    pub fn with_window(
        manifest_id: u16,
        patient_pubkey: [u8; 32],
        guardian_pubkey: [u8; 32],
        window_us: u64,
    ) -> Self {
        Self {
            inner: ConsentMachine::new(manifest_id, patient_pubkey),
            guardian_pubkey,
            pending: None,
            window_us,
        }
    }

    /// Current consent state.
    pub fn state(&self) -> ConsentState {
        self.inner.state()
    }

    /// The manifest ID this machine is bound to.
    pub fn manifest_id(&self) -> u16 {
        self.inner.manifest_id()
    }

    /// The co-authorisation window, in microseconds.
    pub fn window_us(&self) -> u64 {
        self.window_us
    }

    /// If a safety-increasing transition is half-authorised, returns the party
    /// that has already authorised it; otherwise `None`.
    pub fn pending(&self) -> Option<Party> {
        match self.pending {
            Some(p) => Some(p.proposer),
            None => None,
        }
    }

    /// Propose a transition on behalf of `party`.
    ///
    /// The event's truncated signature is verified against the key belonging to
    /// the claimed `party` (the patient key for [`Party::Patient`], the guardian
    /// key for [`Party::Guardian`]).
    ///
    /// * Exposure-*reducing* transitions (`→ Suspended`, `→ Withdrawn`) and
    ///   idempotent re-applications apply immediately and clear any pending
    ///   co-authorisation.
    /// * The exposure-*increasing* transition (`Suspended → Granted`) is
    ///   recorded as pending on the first authorisation and committed only when
    ///   the *other* party authorises the same transition within
    ///   [`Self::window_us`].
    pub fn propose(
        &mut self,
        event: ConsentEvent,
        party: Party,
    ) -> Result<CoAuthOutcome, ConsentError> {
        if event.manifest_id != self.manifest_id() {
            return Err(ConsentError::ManifestMismatch);
        }

        // Verify against the key belonging to the claimed party. A guardian
        // event signed with the patient key (or vice versa) fails here.
        let key: &[u8; 32] = match party {
            Party::Patient => self.inner.trusted_path_pubkey(),
            Party::Guardian => &self.guardian_pubkey,
        };
        verify_truncated(&event, key)?;

        let target = ConsentState::from_u8(event.state)?;
        let current = self.state();

        if !is_admissible_transition(current, target) {
            return Err(ConsentError::InadmissibleTransition);
        }

        if is_exposure_increasing(current, target) {
            return self.handle_increase(party, event.timestamp_us, target);
        }

        // Reducing or idempotent: apply immediately and clear any pending
        // co-authorisation, since the state has moved.
        self.pending = None;
        let new_state = self.inner.apply_verified(target)?;
        Ok(CoAuthOutcome::Applied(new_state))
    }

    /// Handle the one exposure-increasing transition (`Suspended → Granted`).
    fn handle_increase(
        &mut self,
        party: Party,
        ts_us: u64,
        target: ConsentState,
    ) -> Result<CoAuthOutcome, ConsentError> {
        match self.pending {
            // A pending request from the *other* party exists: commit iff the
            // counter-authorisation is newer and within the window.
            Some(p) if p.proposer != party => {
                let fresh = match ts_us.checked_sub(p.proposed_at_us) {
                    Some(delta) => delta <= self.window_us,
                    None => false, // out-of-order: the second event predates the first
                };
                if fresh {
                    self.pending = None;
                    let new_state = self.inner.apply_verified(target)?;
                    Ok(CoAuthOutcome::Applied(new_state))
                } else {
                    // Stale or out-of-order: discard the stale half and re-arm
                    // from the arriving party. Never commits without two fresh
                    // signatures.
                    self.pending = Some(PendingGrant {
                        proposer: party,
                        proposed_at_us: ts_us,
                    });
                    Ok(CoAuthOutcome::PendingCoAuth(self.state()))
                }
            }
            // No pending request, or the *same* party re-proposing: arm/refresh
            // the pending request. A single party can never self-authorise.
            _ => {
                self.pending = Some(PendingGrant {
                    proposer: party,
                    proposed_at_us: ts_us,
                });
                Ok(CoAuthOutcome::PendingCoAuth(self.state()))
            }
        }
    }
}

/// Is the transition `from → to` exposure-increasing?
///
/// The only exposure-increasing transition is `Suspended → Granted`: it resumes
/// neural-data flow. `Granted → Granted` is idempotent (no increase), and every
/// transition toward `Suspended` or `Withdrawn` reduces exposure.
pub const fn is_exposure_increasing(from: ConsentState, to: ConsentState) -> bool {
    matches!((from, to), (ConsentState::Suspended, ConsentState::Granted))
}

/// Pure co-authorisation predicate: a half-authorised exposure increase commits
/// only when the counter-proposing party is *distinct* from the original
/// proposer.
///
/// This is the security core of dual control, isolated as a pure `const fn` so
/// it can be formally verified independently of the timing window. Verified by
/// the Kani harness `co_authorisation_requires_two_parties`.
pub const fn co_authorisation_complete(pending_proposer: Party, incoming: Party) -> bool {
    (pending_proposer as u8) != (incoming as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::compute_tag;

    const PATIENT: [u8; 32] = [0x11u8; 32];
    const GUARDIAN: [u8; 32] = [0x22u8; 32];
    const MID: u16 = 7;

    /// Build a correctly-tagged event signed by `key`.
    fn signed(state: u8, ts: u64, key: &[u8; 32]) -> ConsentEvent {
        let mut e = ConsentEvent {
            state,
            flags: 0,
            manifest_id: MID,
            timestamp_us: ts,
            sig_truncated: 0,
        };
        e.sig_truncated = compute_tag(&e, key);
        e
    }

    fn machine() -> DualControlMachine {
        DualControlMachine::new(MID, PATIENT, GUARDIAN)
    }

    #[test]
    fn either_party_can_suspend_unilaterally() {
        let mut m = machine();
        let r = m.propose(signed(0x02, 1, &PATIENT), Party::Patient).unwrap();
        assert_eq!(r, CoAuthOutcome::Applied(ConsentState::Suspended));

        let mut m2 = machine();
        let r2 = m2
            .propose(signed(0x02, 1, &GUARDIAN), Party::Guardian)
            .unwrap();
        assert_eq!(r2, CoAuthOutcome::Applied(ConsentState::Suspended));
    }

    #[test]
    fn either_party_can_withdraw_unilaterally() {
        let mut m = machine();
        let r = m
            .propose(signed(0x03, 1, &GUARDIAN), Party::Guardian)
            .unwrap();
        assert_eq!(r, CoAuthOutcome::Applied(ConsentState::Withdrawn));
        assert!(m.state().is_terminal());
    }

    #[test]
    fn resume_requires_both_parties() {
        let mut m = machine();
        // Suspend first.
        m.propose(signed(0x02, 1, &PATIENT), Party::Patient).unwrap();
        // Patient alone only arms a pending request.
        let r = m.propose(signed(0x01, 2, &PATIENT), Party::Patient).unwrap();
        assert_eq!(r, CoAuthOutcome::PendingCoAuth(ConsentState::Suspended));
        assert_eq!(m.pending(), Some(Party::Patient));
        assert_eq!(m.state(), ConsentState::Suspended);
        // Guardian completes within the window.
        let r = m
            .propose(signed(0x01, 3, &GUARDIAN), Party::Guardian)
            .unwrap();
        assert_eq!(r, CoAuthOutcome::Applied(ConsentState::Granted));
        assert_eq!(m.pending(), None);
    }

    #[test]
    fn single_party_can_never_resume() {
        let mut m = machine();
        m.propose(signed(0x02, 1, &PATIENT), Party::Patient).unwrap();
        // Many repeated patient proposals never commit.
        for ts in 2..50 {
            let r = m
                .propose(signed(0x01, ts, &PATIENT), Party::Patient)
                .unwrap();
            assert_eq!(r, CoAuthOutcome::PendingCoAuth(ConsentState::Suspended));
            assert_eq!(m.state(), ConsentState::Suspended);
        }
    }

    #[test]
    fn expired_counter_authorisation_does_not_commit() {
        let mut m = DualControlMachine::with_window(MID, PATIENT, GUARDIAN, 100);
        m.propose(signed(0x02, 1, &PATIENT), Party::Patient).unwrap();
        // Patient arms at t=10.
        m.propose(signed(0x01, 10, &PATIENT), Party::Patient).unwrap();
        // Guardian arrives at t=10_000 — far outside the 100µs window.
        let r = m
            .propose(signed(0x01, 10_000, &GUARDIAN), Party::Guardian)
            .unwrap();
        assert_eq!(r, CoAuthOutcome::PendingCoAuth(ConsentState::Suspended));
        // The pending request is now the guardian's, re-armed.
        assert_eq!(m.pending(), Some(Party::Guardian));
        assert_eq!(m.state(), ConsentState::Suspended);
    }

    #[test]
    fn guardian_event_with_patient_key_is_rejected() {
        let mut m = machine();
        m.propose(signed(0x02, 1, &PATIENT), Party::Patient).unwrap();
        // Event tagged with the patient key but claimed as Guardian: signature
        // verification uses the guardian key and fails.
        let forged = signed(0x01, 2, &PATIENT);
        assert!(matches!(
            m.propose(forged, Party::Guardian),
            Err(ConsentError::SignatureInvalid)
        ));
    }

    #[test]
    fn manifest_mismatch_rejected() {
        let mut m = machine();
        let mut e = signed(0x02, 1, &PATIENT);
        e.manifest_id = MID + 1;
        e.sig_truncated = compute_tag(&e, &PATIENT);
        assert!(matches!(
            m.propose(e, Party::Patient),
            Err(ConsentError::ManifestMismatch)
        ));
    }

    #[test]
    fn withdrawn_is_terminal_even_under_dual_control() {
        let mut m = machine();
        m.propose(signed(0x03, 1, &PATIENT), Party::Patient).unwrap();
        // Both parties cannot resurrect a withdrawn manifest.
        assert!(matches!(
            m.propose(signed(0x01, 2, &PATIENT), Party::Patient),
            Err(ConsentError::InadmissibleTransition)
        ));
        assert!(matches!(
            m.propose(signed(0x01, 3, &GUARDIAN), Party::Guardian),
            Err(ConsentError::InadmissibleTransition)
        ));
    }

    #[test]
    fn co_authorisation_complete_is_distinctness() {
        assert!(co_authorisation_complete(Party::Patient, Party::Guardian));
        assert!(co_authorisation_complete(Party::Guardian, Party::Patient));
        assert!(!co_authorisation_complete(Party::Patient, Party::Patient));
        assert!(!co_authorisation_complete(Party::Guardian, Party::Guardian));
    }

    #[test]
    fn exposure_classification() {
        use ConsentState::{Granted, Suspended, Withdrawn};
        assert!(is_exposure_increasing(Suspended, Granted));
        assert!(!is_exposure_increasing(Granted, Granted));
        assert!(!is_exposure_increasing(Granted, Suspended));
        assert!(!is_exposure_increasing(Granted, Withdrawn));
        assert!(!is_exposure_increasing(Suspended, Withdrawn));
    }
}
