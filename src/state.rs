//! Consent state machine per MMP Consent Extension v0.1.0, Section 4.
//!
//! ```text
//!  ┌─────────┐  consent-suspend  ┌───────────┐
//!  │ GRANTED │ ────────────────> │ SUSPENDED │
//!  │         │ <──────────────── │           │
//!  └────┬────┘  consent-resume   └─────┬─────┘
//!       │                              │
//!       │  consent-withdraw            │  consent-withdraw
//!       v                              v
//!  ┌──────────────────────────────────────┐
//!  │            WITHDRAWN                  │
//!  │     (terminal — connection closed)     │
//!  └──────────────────────────────────────┘
//! ```

/// Per-peer consent state. Unilateral — only the owning node can transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConsentState {
    /// Default. Normal MMP coupling evaluation applies.
    Granted = 0x00,

    /// Coupling paused, connection maintained. No cognitive state exchange.
    /// Resumable without re-handshake.
    Suspended = 0x01,

    /// Terminal. All coupling ceased, connection closed.
    /// Re-connection requires new handshake, starts in Granted.
    Withdrawn = 0x02,
}

/// Transition errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionError {
    /// Cannot transition from Withdrawn (terminal state).
    AlreadyWithdrawn,
    /// Cannot resume — not currently suspended.
    NotSuspended,
    /// Cannot suspend — already suspended (idempotent no-op, not an error in protocol).
    AlreadySuspended,
    /// Peer not found in the consent engine peer table.
    PeerNotFound,
}

impl ConsentState {
    /// Attempt to suspend coupling. Idempotent: double-suspend is a no-op.
    pub fn suspend(self) -> Result<ConsentState, TransitionError> {
        match self {
            ConsentState::Granted => Ok(ConsentState::Suspended),
            ConsentState::Suspended => Ok(ConsentState::Suspended), // idempotent
            ConsentState::Withdrawn => Err(TransitionError::AlreadyWithdrawn),
        }
    }

    /// Attempt to resume coupling from suspended state.
    pub fn resume(self) -> Result<ConsentState, TransitionError> {
        match self {
            ConsentState::Suspended => Ok(ConsentState::Granted),
            ConsentState::Granted => Ok(ConsentState::Granted), // idempotent
            ConsentState::Withdrawn => Err(TransitionError::AlreadyWithdrawn),
        }
    }

    /// Withdraw consent. Always succeeds from Granted or Suspended. Terminal.
    pub fn withdraw(self) -> Result<ConsentState, TransitionError> {
        match self {
            ConsentState::Granted | ConsentState::Suspended => Ok(ConsentState::Withdrawn),
            ConsentState::Withdrawn => Err(TransitionError::AlreadyWithdrawn),
        }
    }

    /// Compact 2-bit encoding for BLE gossip (Section 6.4).
    /// 00 = Granted, 01 = Suspended, 10 = Withdrawn, 11 = reserved.
    pub fn to_gossip_bits(self) -> u8 {
        match self {
            ConsentState::Granted => 0b00,
            ConsentState::Suspended => 0b01,
            ConsentState::Withdrawn => 0b10,
        }
    }

    /// Decode from 2-bit gossip encoding.
    pub fn from_gossip_bits(bits: u8) -> Option<Self> {
        match bits & 0b11 {
            0b00 => Some(ConsentState::Granted),
            0b01 => Some(ConsentState::Suspended),
            0b10 => Some(ConsentState::Withdrawn),
            _ => None, // 0b11 = reserved
        }
    }

    /// Whether cognitive frames should be filtered (Section 6.1).
    pub fn allows_cognitive_frames(self) -> bool {
        matches!(self, ConsentState::Granted)
    }
}
