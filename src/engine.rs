//! ConsentEngine — per-peer consent state machine with enforcement.
//!
//! Implements the normative enforcement sequence from Section 5.1:
//!
//! 1. Set local consent state to WITHDRAWN
//! 2. [Safety-critical] Persist to NVRAM (ATECC608B)
//! 3. Remove peer from coupling engine
//! 4. Discard buffered frames from peer
//! 5. Send consent-withdraw notification (best-effort)
//! 6. Close transport connection
//!
//! Steps 1–4 are LOCAL and complete BEFORE step 5.

use crate::state::{ConsentState, TransitionError};
use crate::reason::ReasonCode;

/// Maximum number of peers tracked simultaneously.
/// BLE mesh constraint: N ≤ 8 per MMP timing contract.
pub const MAX_PEERS: usize = 8;

/// Opaque peer identifier (maps to MMP nodeId).
pub type PeerId = [u8; 16]; // UUID v4

/// Per-peer consent record.
#[derive(Debug, Clone)]
pub struct PeerConsent {
    pub peer_id: PeerId,
    pub state: ConsentState,
    pub last_reason: Option<ReasonCode>,
    pub last_transition_us: u64,
}

/// The consent engine. Maintains per-peer state, enforces transitions.
///
/// Designed for `#![no_std]` — fixed-size array, no heap allocation.
pub struct ConsentEngine {
    peers: [Option<PeerConsent>; MAX_PEERS],
    /// Callback: invoked on withdrawal to trigger StimGuard lockout.
    /// In AxonOS, this is `StimGuard::consent_withdrawn()`.
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

    /// Register a StimGuard callback for withdrawal enforcement.
    #[cfg(feature = "stim-guard")]
    pub fn set_withdraw_callback(&mut self, cb: fn(peer_id: &PeerId)) {
        self.on_withdraw = Some(cb);
    }

    /// Register a new peer (on handshake). Starts in Granted state.
    pub fn register_peer(&mut self, peer_id: PeerId, now_us: u64) -> Result<(), &'static str> {
        // Check for duplicate registration
        if self.find_peer(&peer_id).is_some() {
            return Err("peer already registered");
        }
        for slot in self.peers.iter_mut() {
            if slot.is_none() {
                *slot = Some(PeerConsent {
                    peer_id,
                    state: ConsentState::Granted,
                    last_reason: None,
                    last_transition_us: now_us,
                });
                return Ok(());
            }
        }
        Err("peer table full")
    }

    /// Get current consent state for a peer.
    pub fn get_state(&self, peer_id: &PeerId) -> Option<ConsentState> {
        self.find_peer(peer_id).map(|p| p.state)
    }

    /// Suspend coupling with a peer. Section 3.2.
    pub fn suspend(
        &mut self,
        peer_id: &PeerId,
        reason: Option<ReasonCode>,
        now_us: u64,
    ) -> Result<ConsentState, TransitionError> {
        let peer = self.find_peer_mut(peer_id)
            .ok_or(TransitionError::PeerNotFound)?;
        let new_state = peer.state.suspend()?;
        peer.state = new_state;
        peer.last_reason = reason;
        peer.last_transition_us = now_us;
        Ok(new_state)
    }

    /// Resume coupling with a peer. Section 3.3.
    pub fn resume(
        &mut self,
        peer_id: &PeerId,
        now_us: u64,
    ) -> Result<ConsentState, TransitionError> {
        let peer = self.find_peer_mut(peer_id)
            .ok_or(TransitionError::PeerNotFound)?;
        let new_state = peer.state.resume()?;
        peer.state = new_state;
        peer.last_transition_us = now_us;
        Ok(new_state)
    }

    /// Withdraw consent. Section 5.1 enforcement sequence.
    ///
    /// This is the CRITICAL PATH for BCI safety.
    /// Steps 1–4 execute atomically in Secure World when stim-guard is enabled.
    ///
    /// WCET target: <1µs for steps 1–4 on STM32H573.
    pub fn withdraw(
        &mut self,
        peer_id: &PeerId,
        reason: Option<ReasonCode>,
        now_us: u64,
    ) -> Result<ConsentState, TransitionError> {
        let peer = self.find_peer_mut(peer_id)
            .ok_or(TransitionError::PeerNotFound)?;

        // Step 1: Set local consent state to WITHDRAWN
        let new_state = peer.state.withdraw()?;
        peer.state = new_state;
        peer.last_reason = reason;
        peer.last_transition_us = now_us;

        // Step 2: [Safety-critical] NVRAM persistence would happen here
        // In AxonOS: ATECC608B secure element write
        // (platform-specific, not in this crate)

        // Step 3: Remove peer from coupling engine → StimGuard lockout
        #[cfg(feature = "stim-guard")]
        if let Some(cb) = self.on_withdraw {
            cb(peer_id);
        }

        // Step 4: Discard buffered frames (caller responsibility)
        // Steps 5–6: Notification + connection close (caller responsibility, async)

        Ok(new_state)
    }

    /// Withdraw ALL peers. scope: "all" per Section 3.1.
    ///
    /// Local enforcement completes before any notification is sent.
    pub fn withdraw_all(
        &mut self,
        reason: Option<ReasonCode>,
        now_us: u64,
    ) -> usize {
        let mut count = 0;
        for slot in self.peers.iter_mut() {
            if let Some(peer) = slot {
                if peer.state != ConsentState::Withdrawn {
                    peer.state = ConsentState::Withdrawn;
                    peer.last_reason = reason;
                    peer.last_transition_us = now_us;

                    #[cfg(feature = "stim-guard")]
                    if let Some(cb) = self.on_withdraw {
                        cb(&peer.peer_id);
                    }

                    count += 1;
                }
            }
        }
        count
    }

    /// Check whether cognitive frames from a peer should be processed.
    /// Section 6.1: if consent is not Granted, cognitive frames are silently discarded.
    pub fn allows_cognitive_frames(&self, peer_id: &PeerId) -> bool {
        self.find_peer(peer_id)
            .map(|p| p.state.allows_cognitive_frames())
            .unwrap_or(false) // unknown peer → reject
    }

    /// Generate 2-bit gossip encoding for all peers. Section 6.4.
    /// Returns (peer_id, 2-bit state) pairs for peer-info frame.
    pub fn gossip_snapshot(&self) -> impl Iterator<Item = (&PeerId, u8)> {
        self.peers.iter()
            .filter_map(|slot| slot.as_ref())
            .map(|peer| (&peer.peer_id, peer.state.to_gossip_bits()))
    }

    // --- Internal helpers ---

    fn find_peer(&self, peer_id: &PeerId) -> Option<&PeerConsent> {
        self.peers.iter()
            .filter_map(|slot| slot.as_ref())
            .find(|p| &p.peer_id == peer_id)
    }

    fn find_peer_mut(&mut self, peer_id: &PeerId) -> Option<&mut PeerConsent> {
        self.peers.iter_mut()
            .filter_map(|slot| slot.as_mut())
            .find(|p| &p.peer_id == peer_id)
    }
}
