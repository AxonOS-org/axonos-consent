//! Consent state machine per MMP Consent Extension v0.1.0, Section 4.

/// Per-peer consent state. Unilateral — only the owning node can transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConsentState {
    Granted = 0x00,
    Suspended = 0x01,
    Withdrawn = 0x02,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionError {
    AlreadyWithdrawn,
    NotSuspended,
    AlreadySuspended,
    PeerNotFound,
}

impl ConsentState {
    pub fn suspend(self) -> Result<ConsentState, TransitionError> {
        match self {
            Self::Granted => Ok(Self::Suspended),
            Self::Suspended => Ok(Self::Suspended), // idempotent
            Self::Withdrawn => Err(TransitionError::AlreadyWithdrawn),
        }
    }

    pub fn resume(self) -> Result<ConsentState, TransitionError> {
        match self {
            Self::Suspended => Ok(Self::Granted),
            Self::Granted => Ok(Self::Granted), // idempotent
            Self::Withdrawn => Err(TransitionError::AlreadyWithdrawn),
        }
    }

    pub fn withdraw(self) -> Result<ConsentState, TransitionError> {
        match self {
            Self::Granted | Self::Suspended => Ok(Self::Withdrawn),
            Self::Withdrawn => Err(TransitionError::AlreadyWithdrawn),
        }
    }

    pub fn to_gossip_bits(self) -> u8 {
        self as u8
    }

    pub fn from_gossip_bits(bits: u8) -> Option<Self> {
        match bits & 0b11 {
            0 => Some(Self::Granted), 1 => Some(Self::Suspended),
            2 => Some(Self::Withdrawn), _ => None,
        }
    }

    pub fn allows_cognitive_frames(self) -> bool {
        matches!(self, Self::Granted)
    }
}
