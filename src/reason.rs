//! Reason code registry per MMP Consent Extension v0.1.0, Section 3.4.
//!
//! Codes 0x00–0x0F: reserved by specification.
//! Codes 0x10–0xFF: implementation-specific.

/// Machine-readable reason for consent transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReasonCode {
    // === Spec-reserved (0x00–0x0F) ===

    /// No reason given.
    Unspecified = 0x00,
    /// User or operator requested withdrawal.
    UserInitiated = 0x01,
    /// Safety constraint violated.
    SafetyViolation = 0x02,
    /// Hardware-level fault detected.
    HardwareFault = 0x03,

    // === AxonOS implementation-specific (0x10–0xFF) ===

    /// StimGuard triggered lockout — withdrawal initiated by Cognitive Hypervisor,
    /// not by user. Indicates repeated charge density violations or thermal exceedance.
    StimGuardLockout = 0x10,
    /// Session attestation failure — neural authentication mismatch detected.
    SessionAttestationFailure = 0x11,
    /// Physical emergency button pressed — hardware interrupt bypass.
    EmergencyButton = 0x12,
    /// Swarm fault detector flagged Byzantine node behaviour.
    SwarmFaultDetected = 0x13,
}

impl ReasonCode {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0x00 => ReasonCode::Unspecified,
            0x01 => ReasonCode::UserInitiated,
            0x02 => ReasonCode::SafetyViolation,
            0x03 => ReasonCode::HardwareFault,
            0x10 => ReasonCode::StimGuardLockout,
            0x11 => ReasonCode::SessionAttestationFailure,
            0x12 => ReasonCode::EmergencyButton,
            0x13 => ReasonCode::SwarmFaultDetected,
            _ => ReasonCode::Unspecified, // unknown codes default to unspecified
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }

    /// Whether this code is in the spec-reserved range (0x00–0x0F).
    pub fn is_spec_reserved(self) -> bool {
        (self as u8) <= 0x0F
    }

    /// Whether this code is AxonOS implementation-specific (0x10–0xFF).
    pub fn is_implementation_specific(self) -> bool {
        (self as u8) >= 0x10
    }
}
