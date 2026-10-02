// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! Typed refusals.
//!
//! Every way a frame can be refused has its own variant, and none carries
//! dynamic data, so the type is `Copy` and refusing never allocates. The order
//! in which the checks run is normative (SPEC §7.6): a frame is refused for the
//! first rule it breaks, so two implementations refuse the same frame for the
//! same reason.

/// Why a frame, a key or a configuration was refused.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum ConsentError {
    /// The frame is not exactly [`FRAME_LEN`](crate::wire::FRAME_LEN) bytes (SPEC §6.1).
    WireFormatLength,
    /// The record does not begin with the `AXC2` magic (SPEC §6.1).
    BadMagic,
    /// The state byte is not one of the three discriminants (SPEC §2.1).
    ReservedDiscriminant,
    /// A reserved bit is set in the flags byte (SPEC §6.2).
    ReservedFlagBit,
    /// The terminal flag disagrees with the state: it must be set exactly when
    /// the state is `Withdrawn` (SPEC §6.2).
    TerminalFlagMismatch,
    /// A reserved byte of the record is not zero (SPEC §6.1).
    ReservedFieldNonZero,
    /// The signature does not verify under the signer's key, or names a signer
    /// this machine has no key for (SPEC §7).
    SignatureInvalid,
    /// The sequence number is not greater than the last one consumed from this
    /// signer (SPEC §7.5).
    Replay,
    /// The record names another manifest installation (SPEC §7.6).
    ManifestMismatch,
    /// The transition from the current state is not admissible (SPEC §3.2).
    InadmissibleTransition,
    /// A public key was refused as a trust anchor: not a valid point, or of
    /// small order (SPEC §7.4).
    KeyInvalid,
    /// The machine was configured in a way the specification forbids, such as
    /// the same key for patient and guardian, or a zero co-authorisation window
    /// (SPEC §12.2, §12.4).
    ConfigurationInvalid,
}

impl core::fmt::Display for ConsentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::WireFormatLength => "frame is not exactly 96 bytes",
            Self::BadMagic => "record does not begin with the AXC2 magic",
            Self::ReservedDiscriminant => "reserved state discriminant",
            Self::ReservedFlagBit => "reserved flag bit set",
            Self::TerminalFlagMismatch => "terminal flag disagrees with the state",
            Self::ReservedFieldNonZero => "reserved field is not zero",
            Self::SignatureInvalid => "signature does not verify",
            Self::Replay => "sequence number already consumed",
            Self::ManifestMismatch => "record names another manifest",
            Self::InadmissibleTransition => "inadmissible transition",
            Self::KeyInvalid => "public key refused as a trust anchor",
            Self::ConfigurationInvalid => "configuration forbidden by the specification",
        })
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ConsentError {}

impl ConsentError {
    /// The byte transmitted at the kernel/SDK boundary (AxonOS Standard, Section 20).
    ///
    /// Shape refusals map to `0x07`, authentication refusals (including replay)
    /// to `0x08`, and refusals that should never reach an application to `0xFF`.
    pub const fn to_abi_code(&self) -> u8 {
        match self {
            Self::WireFormatLength
            | Self::BadMagic
            | Self::ReservedDiscriminant
            | Self::ReservedFlagBit
            | Self::TerminalFlagMismatch
            | Self::ReservedFieldNonZero => 0x07,
            Self::SignatureInvalid | Self::Replay | Self::KeyInvalid => 0x08,
            Self::ManifestMismatch | Self::InadmissibleTransition | Self::ConfigurationInvalid => {
                0xFF
            }
        }
    }
}
