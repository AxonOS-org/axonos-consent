//! Typed error taxonomy for the consent subsystem.
//!
//! Each variant maps to one error code at the kernel/SDK boundary per SPEC §10.
//! No variant carries dynamic data — this keeps the type `Copy` and ensures
//! zero allocation on the critical path.

/// Errors that can occur during consent event processing.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ConsentError {
    /// Wire-format input was not exactly 16 bytes (SPEC §6).
    WireFormatLength,
    /// Reserved bit set in the flags byte (SPEC §6.2).
    ReservedFlagBit,
    /// State byte was not one of the three known discriminants (SPEC §2.1).
    ReservedDiscriminant,
    /// Signature verification failed (SPEC §7).
    SignatureInvalid,
    /// Event manifest ID does not match this machine's manifest ID.
    ManifestMismatch,
    /// Transition from current state to target state is not admissible (SPEC §3.2).
    InadmissibleTransition,
    /// CBOR decoder bound violation (depth or length).
    CborBoundViolation,
}

impl core::fmt::Display for ConsentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::WireFormatLength => f.write_str("wire-format length not exactly 16 bytes"),
            Self::ReservedFlagBit => f.write_str("reserved flag bit set"),
            Self::ReservedDiscriminant => f.write_str("reserved state discriminant"),
            Self::SignatureInvalid => f.write_str("signature verification failed"),
            Self::ManifestMismatch => f.write_str("event manifest ID does not match"),
            Self::InadmissibleTransition => f.write_str("inadmissible transition"),
            Self::CborBoundViolation => f.write_str("CBOR depth or length bound violation"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ConsentError {}

/// Convert to the kernel/SDK boundary error code per AxonOS Standard §7.4.
impl ConsentError {
    /// Map to the byte-level error code transmitted at the kernel ABI boundary.
    pub fn to_abi_code(&self) -> u8 {
        match self {
            // 0x07 ReservedFieldNonZero — for both reserved-bit and reserved-discriminant
            Self::ReservedFlagBit | Self::ReservedDiscriminant => 0x07,
            // 0x08 SignatureInvalid
            Self::SignatureInvalid => 0x08,
            // 0xFF InternalError — for transitions that should not have reached the SDK
            Self::ManifestMismatch | Self::InadmissibleTransition => 0xFF,
            // 0x07 — wire-format-shape errors
            Self::WireFormatLength | Self::CborBoundViolation => 0x07,
        }
    }
}
