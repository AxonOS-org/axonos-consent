//! Consent frame types per MMP Consent Extension v0.1.0, Section 3.
//!
//! Frame types use string identifiers per MMP Section 7 (frame registry):
//! - "consent-withdraw"
//! - "consent-suspend"
//! - "consent-resume"

use crate::reason::ReasonCode;

/// Section 3.1: consent-withdraw
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentWithdraw {
    /// "peer" (single peer) or "all" (all peers).
    pub scope: Scope,
    /// Machine-readable reason code (Section 3.4). Optional.
    pub reason_code: Option<ReasonCode>,
    /// Human-readable explanation. Not used for protocol decisions. Optional.
    pub reason: Option<alloc::string::String>,
    /// Swarm epoch index k. Optional (real-time implementations).
    pub epoch: Option<u64>,
    /// Unix milliseconds. Optional.
    pub timestamp_ms: Option<u64>,
    /// Unix microseconds. When present, takes precedence over timestamp_ms.
    pub timestamp_us: Option<u64>,
}

/// Section 3.2: consent-suspend
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentSuspend {
    pub reason_code: Option<ReasonCode>,
    pub reason: Option<alloc::string::String>,
    pub timestamp_ms: Option<u64>,
    pub timestamp_us: Option<u64>,
}

/// Section 3.3: consent-resume
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentResume {
    pub timestamp_ms: Option<u64>,
    pub timestamp_us: Option<u64>,
}

/// Scope for consent-withdraw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Withdraw consent for the receiving peer only.
    Peer,
    /// Withdraw consent for all peers. Delivery is asynchronous.
    All,
}

impl Scope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Scope::Peer => "peer",
            Scope::All => "all",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "peer" => Some(Scope::Peer),
            "all" => Some(Scope::All),
            _ => None,
        }
    }
}

/// Unified consent frame enum for codec dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsentFrame {
    Withdraw(ConsentWithdraw),
    Suspend(ConsentSuspend),
    Resume(ConsentResume),
}

impl ConsentFrame {
    /// MMP frame type string identifier (Section 7, frame registry).
    pub fn type_str(&self) -> &'static str {
        match self {
            ConsentFrame::Withdraw(_) => "consent-withdraw",
            ConsentFrame::Suspend(_) => "consent-suspend",
            ConsentFrame::Resume(_) => "consent-resume",
        }
    }
}
