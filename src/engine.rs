//! ConsentEngine — per-peer consent state machine with enforcement.
//! Section 5.1 enforcement sequence. Fully no_std, zero-alloc.

use crate::state::{ConsentState, TransitionError};
use crate::reason::ReasonCode;

pub const MAX_PEERS: usize = 8;
pub type PeerId = [u8; 16];

#[derive(Debug, Clone)]
pub struct PeerConsent {
    pub peer_id: PeerId,
    pub state: ConsentState,
    pub last_reason: Option<ReasonCode>,
    pub last_transition_us: u64,
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
        if self.find_peer(&peer_id).is_some() {
            return Err("peer already registered");
        }
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

    /// Withdraw consent. CRITICAL PATH. Steps 1-4 atomic in Secure World.
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
