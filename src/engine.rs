//! ConsentEngine — per-peer state machine with mandatory invariant enforcement.
//!
//! §5.1 enforcement sequence. Zero-alloc. Fixed peer table.
//!
//! ## Entry point
//!
//! `process_frame()` is the **single entry point** for all consent frames.
//! It enforces the full validation pipeline:
//!
//! 1. `invariants::check_frame()` — MUST/SHOULD validation
//! 2. `state.apply_frame()` — exhaustive transition check
//! 3. State update + StimGuard callback (if withdrawal)
//!
//! Direct `suspend()/resume()/withdraw()` methods exist for internal use
//! (e.g., emergency button bypass) but skip frame-level validation.

use crate::state::{ConsentState, TransitionError};
use crate::reason::ReasonCode;
use crate::frames::ConsentFrame;
use crate::invariants;
use crate::error::Error;

/// Maximum peers. BLE mesh constraint. §6.4.
pub const MAX_PEERS: usize = 8;

/// Opaque peer identifier (MMP nodeId, UUID v4).
pub type PeerId = [u8; 16];

#[derive(Debug, Clone)]
pub struct PeerConsent {
    pub peer_id: PeerId,
    pub state: ConsentState,
    pub last_reason: Option<ReasonCode>,
    pub last_transition_us: u64,
}

/// Processing result with optional warnings.
#[derive(Debug)]
pub struct ProcessResult {
    pub new_state: ConsentState,
    pub warnings: [Option<invariants::InvariantWarning>; 4],
    pub warning_count: u8,
}

pub struct ConsentEngine {
    peers: [Option<PeerConsent>; MAX_PEERS],
    #[cfg(feature = "stim-guard")]
    on_withdraw: Option<fn(peer_id: &PeerId)>,
}

impl ConsentEngine {
    pub const fn new() -> Self {
        const NONE: Option<PeerConsent> = None;
        Self {
            peers: [NONE; MAX_PEERS],
            #[cfg(feature = "stim-guard")]
            on_withdraw: None,
        }
    }

    #[cfg(feature = "stim-guard")]
    pub fn set_withdraw_callback(&mut self, cb: fn(peer_id: &PeerId)) {
        self.on_withdraw = Some(cb);
    }

    pub fn register_peer(&mut self, peer_id: PeerId, now_us: u64) -> Result<(), &'static str> {
        if self.find_peer(&peer_id).is_some() { return Err("peer already registered"); }
        for slot in self.peers.iter_mut() {
            if slot.is_none() {
                *slot = Some(PeerConsent {
                    peer_id, state: ConsentState::Granted,
                    last_reason: None, last_transition_us: now_us,
                });
                return Ok(());
            }
        }
        Err("peer table full")
    }

    pub fn get_state(&self, peer_id: &PeerId) -> Option<ConsentState> {
        self.find_peer(peer_id).map(|p| p.state)
    }

    /// **Primary entry point.** Process an incoming consent frame with full validation.
    ///
    /// Pipeline:
    /// 1. Check frame invariants (MUST violations → reject, SHOULD → warn)
    /// 2. Check state transition legality (WITHDRAWN → any = reject)
    /// 3. Apply state transition
    /// 4. Trigger StimGuard on withdrawal (if feature enabled)
    ///
    /// WCET: O(1) — fixed field checks + single state match. <1µs on M4F.
    pub fn process_frame(
        &mut self,
        peer_id: &PeerId,
        frame: &ConsentFrame,
        reason: Option<ReasonCode>,
        now_us: u64,
    ) -> Result<ProcessResult, Error> {
        // Step 1: Frame-level invariant check
        let inv = invariants::check_frame(frame);
        if !inv.is_valid() {
            // Return first violation
            return Err(Error::Invariant(
                inv.violations[0].unwrap() // safe: violation_count > 0
            ));
        }

        // Step 2: Find peer + check transition legality
        let peer = self.find_peer_mut(peer_id)
            .ok_or(Error::Transition(TransitionError::PeerNotFound))?;

        let new_state = peer.state.apply_frame(frame)
            .map_err(Error::Transition)?;

        // Step 3: Apply transition
        peer.state = new_state;
        peer.last_reason = reason;
        peer.last_transition_us = now_us;

        // Step 4: StimGuard callback on withdrawal
        if new_state == ConsentState::Withdrawn {
            #[cfg(feature = "stim-guard")]
            if let Some(cb) = self.on_withdraw { cb(peer_id); }
        }

        Ok(ProcessResult {
            new_state,
            warnings: inv.warnings,
            warning_count: inv.warning_count,
        })
    }

    // --- Direct methods (bypass frame validation, for internal/emergency use) ---

    pub fn suspend(&mut self, peer_id: &PeerId, reason: Option<ReasonCode>, now_us: u64)
        -> Result<ConsentState, TransitionError>
    {
        let peer = self.find_peer_mut(peer_id).ok_or(TransitionError::PeerNotFound)?;
        let s = peer.state.suspend()?;
        peer.state = s; peer.last_reason = reason; peer.last_transition_us = now_us;
        Ok(s)
    }

    pub fn resume(&mut self, peer_id: &PeerId, now_us: u64)
        -> Result<ConsentState, TransitionError>
    {
        let peer = self.find_peer_mut(peer_id).ok_or(TransitionError::PeerNotFound)?;
        let s = peer.state.resume()?;
        peer.state = s; peer.last_transition_us = now_us;
        Ok(s)
    }

    /// Direct withdrawal. Used by emergency button (bypasses frame validation).
    /// §8: physical button → direct interrupt → this function.
    ///
    /// WCET: state write + optional StimGuard callback. <1µs on M4F.
    pub fn withdraw(&mut self, peer_id: &PeerId, reason: Option<ReasonCode>, now_us: u64)
        -> Result<ConsentState, TransitionError>
    {
        let peer = self.find_peer_mut(peer_id).ok_or(TransitionError::PeerNotFound)?;
        let s = peer.state.withdraw()?;
        peer.state = s; peer.last_reason = reason; peer.last_transition_us = now_us;
        #[cfg(feature = "stim-guard")]
        if let Some(cb) = self.on_withdraw { cb(peer_id); }
        Ok(s)
    }

    pub fn withdraw_all(&mut self, reason: Option<ReasonCode>, now_us: u64) -> usize {
        let mut count = 0;
        for slot in self.peers.iter_mut() {
            if let Some(peer) = slot {
                if peer.state != ConsentState::Withdrawn {
                    peer.state = ConsentState::Withdrawn;
                    peer.last_reason = reason; peer.last_transition_us = now_us;
                    #[cfg(feature = "stim-guard")]
                    if let Some(cb) = self.on_withdraw { cb(&peer.peer_id); }
                    count += 1;
                }
            }
        }
        count
    }

    /// §6.1: check if cognitive frames should be processed for this peer.
    pub fn allows_cognitive_frames(&self, peer_id: &PeerId) -> bool {
        self.find_peer(peer_id).map(|p| p.state.allows_cognitive_frames()).unwrap_or(false)
    }

    fn find_peer(&self, id: &PeerId) -> Option<&PeerConsent> {
        self.peers.iter().filter_map(|s| s.as_ref()).find(|p| &p.peer_id == id)
    }
    fn find_peer_mut(&mut self, id: &PeerId) -> Option<&mut PeerConsent> {
        self.peers.iter_mut().filter_map(|s| s.as_mut()).find(|p| &p.peer_id == id)
    }
}
