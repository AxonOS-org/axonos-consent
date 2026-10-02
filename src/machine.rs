// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! The single-party consent machine (SPEC §2–§9).
//!
//! A frame is admitted in four layers, each with its own property:
//!
//! ```text
//!  frame ──▶ 1 wire      exactly 96 bytes, canonical record          (total, canonical: Kani)
//!        ──▶ 2 auth      strict Ed25519 under the trusted-path key   (no transition without it: Kani)
//!        ──▶ 3 sequence  greater than the last one consumed          (no replay: Kani)
//!        ──▶ 4 FSM       admissible from the current state           (Withdrawn absorbing: Kani)
//!        ──▶ gate        one atomic word holds state and publications (linearizable: loom)
//! ```
//!
//! Transitions take `&mut self`, so they are serialized by the borrow checker.
//! The state itself lives in the [`PublicationGate`], an atomic word, so the
//! kernel's publication path can read it through a shared reference while
//! transitions happen elsewhere.

use crate::auth::{authenticate, SignatureVerifier};
use crate::error::ConsentError;
use crate::gate::PublicationGate;
use crate::state::{is_admissible_transition, ConsentState};

/// What must survive a power cycle (SPEC §8): the state, and the last sequence
/// number consumed from each signer. Losing the sequence numbers would let an
/// old frame be replayed after a reboot.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Persisted {
    /// The consent state.
    pub state: ConsentState,
    /// The last sequence number consumed from the patient.
    pub patient_sequence: u64,
    /// The last sequence number consumed from the guardian (dual control only;
    /// zero otherwise).
    pub guardian_sequence: u64,
}

impl Persisted {
    /// A fresh installation: `Granted`, nothing consumed (SPEC §2.2).
    pub const FRESH: Self = Self {
        state: ConsentState::Granted,
        patient_sequence: 0,
        guardian_sequence: 0,
    };
}

/// The consent machine for one manifest installation and one trusted-path key.
#[derive(Debug)]
pub struct ConsentMachine<V: SignatureVerifier> {
    manifest_id: u16,
    key: [u8; 32],
    verifier: V,
    gate: PublicationGate,
    last_sequence: u64,
}

impl<V: SignatureVerifier> ConsentMachine<V> {
    /// A machine for a freshly installed manifest, in the `Granted` state.
    /// Refuses a key the verifier will not accept as a trust anchor.
    pub fn new(manifest_id: u16, trusted_key: [u8; 32], verifier: V) -> Result<Self, ConsentError> {
        Self::restore(manifest_id, trusted_key, verifier, Persisted::FRESH)
    }

    /// A machine restored from persistent storage (SPEC §8.1).
    pub fn restore(
        manifest_id: u16,
        trusted_key: [u8; 32],
        verifier: V,
        persisted: Persisted,
    ) -> Result<Self, ConsentError> {
        if !verifier.accepts_key(&trusted_key) {
            return Err(ConsentError::KeyInvalid);
        }
        Ok(Self {
            manifest_id,
            key: trusted_key,
            verifier,
            gate: PublicationGate::with_state(persisted.state),
            last_sequence: persisted.patient_sequence,
        })
    }

    /// Admit one frame from the trusted path. On success the new state is
    /// returned and, before this function returns, is the state every
    /// publication sees.
    ///
    /// The checks run in the order of SPEC §7.6. A frame that authenticates and
    /// passes the sequence check consumes its sequence number even if the
    /// transition is then refused, so it can never be replayed into a later,
    /// different state.
    pub fn handle(&mut self, frame: &[u8]) -> Result<ConsentState, ConsentError> {
        let authenticated = authenticate(frame, &self.key, None, &self.verifier)?;
        let record = authenticated.record();
        if record.manifest_id() != self.manifest_id {
            return Err(ConsentError::ManifestMismatch);
        }
        if record.sequence() <= self.last_sequence {
            return Err(ConsentError::Replay);
        }
        self.last_sequence = record.sequence();
        let target = record.state();
        if !is_admissible_transition(self.gate.state(), target) {
            return Err(ConsentError::InadmissibleTransition);
        }
        self.gate.set_state(target);
        Ok(target)
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

    /// The last sequence number consumed.
    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    /// What to write to persistent storage after an admitted frame.
    pub fn persisted(&self) -> Persisted {
        Persisted {
            state: self.state(),
            patient_sequence: self.last_sequence,
            guardian_sequence: 0,
        }
    }
}
