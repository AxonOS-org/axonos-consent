// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! Dual control: a patient and a guardian (SPEC §12).
//!
//! The safe-direction principle: either party can stop the flow alone, and
//! only both together can resume it. Three details make that hold against an
//! adversary rather than only against mistakes:
//!
//! * **The role is signed.** A record declares its party in the flags byte,
//!   inside the signature, and is verified under that party's key. A caller
//!   cannot relabel a frame.
//! * **The keys differ.** A machine refuses to be built with one key for both
//!   parties, which would let one signer authorise twice.
//! * **The window runs on the kernel's clock.** Co-authorisation freshness is
//!   measured with `now_us`, supplied by the kernel's monotonic clock, never
//!   with the timestamp a signer wrote into the record.

use crate::auth::{authenticate, SignatureVerifier};
use crate::error::ConsentError;
use crate::gate::PublicationGate;
use crate::machine::Persisted;
use crate::state::{is_admissible_transition, is_exposure_increasing, ConsentState};
use crate::wire::Party;

/// The default co-authorisation window: two minutes.
pub const DEFAULT_CO_AUTH_WINDOW_US: u64 = 120_000_000;

/// What a proposal did.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum CoAuthOutcome {
    /// The transition took effect; this is the new state.
    Applied(ConsentState),
    /// One party has asked to resume; the state is unchanged until the other
    /// party agrees within the window.
    PendingCoAuth(ConsentState),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Pending {
    proposer: Party,
    armed_at_us: u64,
}

/// The consent machine for a dual-control installation.
#[derive(Debug)]
pub struct DualControlMachine<V: SignatureVerifier> {
    manifest_id: u16,
    patient_key: [u8; 32],
    guardian_key: [u8; 32],
    verifier: V,
    gate: PublicationGate,
    last_sequence: [u64; 2],
    pending: Option<Pending>,
    window_us: u64,
}

impl<V: SignatureVerifier> DualControlMachine<V> {
    /// A machine for a freshly installed manifest with the default window.
    pub fn new(
        manifest_id: u16,
        patient_key: [u8; 32],
        guardian_key: [u8; 32],
        verifier: V,
    ) -> Result<Self, ConsentError> {
        Self::restore(
            manifest_id,
            patient_key,
            guardian_key,
            verifier,
            Persisted::FRESH,
            DEFAULT_CO_AUTH_WINDOW_US,
        )
    }

    /// A machine restored from persistent storage, with an explicit window.
    /// A pending co-authorisation is never persisted: after a power cycle both
    /// parties authorise again, which errs in the safe direction.
    pub fn restore(
        manifest_id: u16,
        patient_key: [u8; 32],
        guardian_key: [u8; 32],
        verifier: V,
        persisted: Persisted,
        window_us: u64,
    ) -> Result<Self, ConsentError> {
        if !verifier.accepts_key(&patient_key) || !verifier.accepts_key(&guardian_key) {
            return Err(ConsentError::KeyInvalid);
        }
        if patient_key == guardian_key || window_us == 0 {
            return Err(ConsentError::ConfigurationInvalid);
        }
        Ok(Self {
            manifest_id,
            patient_key,
            guardian_key,
            verifier,
            gate: PublicationGate::with_state(persisted.state),
            last_sequence: [persisted.patient_sequence, persisted.guardian_sequence],
            pending: None,
            window_us,
        })
    }

    /// Admit one frame. `now_us` is the kernel's monotonic clock at receipt.
    pub fn propose(&mut self, frame: &[u8], now_us: u64) -> Result<CoAuthOutcome, ConsentError> {
        let authenticated = authenticate(
            frame,
            &self.patient_key,
            Some(&self.guardian_key),
            &self.verifier,
        )?;
        let record = authenticated.record();
        if record.manifest_id() != self.manifest_id {
            return Err(ConsentError::ManifestMismatch);
        }
        let party = authenticated.party();
        let slot = &mut self.last_sequence[index(party)];
        if record.sequence() <= *slot {
            return Err(ConsentError::Replay);
        }
        *slot = record.sequence();

        let current = self.gate.state();
        let target = record.state();
        if !is_admissible_transition(current, target) {
            return Err(ConsentError::InadmissibleTransition);
        }
        if !is_exposure_increasing(current, target) {
            // Stopping, pausing or re-asserting: one party is enough, and any
            // pending request to resume is cancelled.
            self.pending = None;
            self.gate.set_state(target);
            return Ok(CoAuthOutcome::Applied(target));
        }
        match self.pending {
            Some(p) if p.proposer != party && within(p.armed_at_us, now_us, self.window_us) => {
                self.pending = None;
                self.gate.set_state(target);
                Ok(CoAuthOutcome::Applied(target))
            }
            _ => {
                // First half, a stale first half, or the same party again:
                // (re)arm from this party. One party can never complete it.
                self.pending = Some(Pending {
                    proposer: party,
                    armed_at_us: now_us,
                });
                Ok(CoAuthOutcome::PendingCoAuth(current))
            }
        }
    }

    /// The current state.
    pub fn state(&self) -> ConsentState {
        self.gate.state()
    }

    /// The gate the kernel's publication path commits through.
    pub fn gate(&self) -> &PublicationGate {
        &self.gate
    }

    /// The manifest installation this machine is bound to.
    pub fn manifest_id(&self) -> u16 {
        self.manifest_id
    }

    /// The co-authorisation window, in microseconds.
    pub fn window_us(&self) -> u64 {
        self.window_us
    }

    /// The party with a pending request to resume, if any.
    pub fn pending(&self) -> Option<Party> {
        self.pending.map(|p| p.proposer)
    }

    /// The last sequence number consumed from `party`.
    pub fn last_sequence(&self, party: Party) -> u64 {
        self.last_sequence[index(party)]
    }

    /// What to write to persistent storage after an admitted frame.
    pub fn persisted(&self) -> Persisted {
        Persisted {
            state: self.state(),
            patient_sequence: self.last_sequence[0],
            guardian_sequence: self.last_sequence[1],
        }
    }
}

const fn index(party: Party) -> usize {
    match party {
        Party::Patient => 0,
        Party::Guardian => 1,
    }
}

/// Is `now` within `window` after `armed`? A clock that reads earlier than the
/// arming instant is treated as stale.
const fn within(armed: u64, now: u64, window: u64) -> bool {
    match now.checked_sub(armed) {
        Some(elapsed) => elapsed <= window,
        None => false,
    }
}
