//! The 16-byte wire format for consent events.
//!
//! See [SPEC §6](https://github.com/AxonOS-org/axonos-consent/blob/main/SPEC.md#6-wire-format)
//! for the normative byte-level definition.

use crate::error::ConsentError;

/// The exact wire-format size, in bytes. Enforced at decoding.
pub const WIRE_SIZE: usize = 16;

/// Maximum CBOR decoding depth (compile-time constant; L1-verified by Kani).
pub const CBOR_MAX_DEPTH: u8 = 8;

/// Maximum CBOR record length, in bytes (compile-time constant; L1-verified by Kani).
pub const CBOR_MAX_LEN: usize = 256;

/// Flag bit 0: target state is terminal (`Withdrawn`).
pub const FLAG_TERMINAL: u8 = 1 << 0;

/// Flag bit 1: event originated from a TrustZone-M Secure-World UI.
pub const FLAG_FROM_SECURE_WORLD: u8 = 1 << 1;

/// Flag bit 2: event is acceptable as an idempotent re-application.
pub const FLAG_REPLAY_TOLERANT: u8 = 1 << 2;

/// Flag bit 3: event originated from the guardian key in a dual-control
/// (multi-party) deployment. See [`crate::dual_control`]. Defined in v0.5.0;
/// reserved and rejected in earlier versions.
pub const FLAG_GUARDIAN: u8 = 1 << 3;

/// Mask of all currently-defined flag bits. Bits outside this mask are reserved.
pub const FLAGS_DEFINED_MASK: u8 =
    FLAG_TERMINAL | FLAG_FROM_SECURE_WORLD | FLAG_REPLAY_TOLERANT | FLAG_GUARDIAN;

/// A consent event as it crosses the trusted-path / kernel boundary.
///
/// 16 bytes on the wire, little-endian. See SPEC §6.1.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ConsentEvent {
    /// Target state discriminant: 0x01 `Granted`, 0x02 `Suspended`, 0x03 `Withdrawn`.
    pub state: u8,

    /// Flag bitfield. See [`FLAG_TERMINAL`], [`FLAG_FROM_SECURE_WORLD`], [`FLAG_REPLAY_TOLERANT`].
    pub flags: u8,

    /// Per-device unique manifest identifier (16-bit; 65,536 concurrent installations).
    pub manifest_id: u16,

    /// Kernel monotonic timestamp at event signing, in microseconds.
    pub timestamp_us: u64,

    /// Truncated Ed25519 signature (4 bytes; collision resistance ~ 2^32 against
    /// accidental corruption only). The full 64-byte signature is verified
    /// out-of-band by the trusted-path crypto path.
    pub sig_truncated: u32,
}

impl ConsentEvent {
    /// Decode a wire-format buffer of exactly [`WIRE_SIZE`] bytes.
    ///
    /// Refuses any other length, any reserved-bit-set flags byte, and any
    /// unknown state discriminant.
    pub fn from_bytes(buf: &[u8]) -> Result<Self, ConsentError> {
        if buf.len() != WIRE_SIZE {
            return Err(ConsentError::WireFormatLength);
        }

        let state = buf[0];
        crate::state::ConsentState::from_u8(state)?;

        let flags = buf[1];
        if flags & !FLAGS_DEFINED_MASK != 0 {
            return Err(ConsentError::ReservedFlagBit);
        }

        let manifest_id = u16::from_le_bytes([buf[2], buf[3]]);
        let timestamp_us = u64::from_le_bytes([
            buf[4], buf[5], buf[6], buf[7], buf[8], buf[9], buf[10], buf[11],
        ]);
        let sig_truncated = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);

        Ok(Self {
            state,
            flags,
            manifest_id,
            timestamp_us,
            sig_truncated,
        })
    }

    /// Encode to a 16-byte buffer (little-endian).
    pub fn to_bytes(&self) -> [u8; WIRE_SIZE] {
        let mut buf = [0u8; WIRE_SIZE];
        buf[0] = self.state;
        buf[1] = self.flags;
        buf[2..4].copy_from_slice(&self.manifest_id.to_le_bytes());
        buf[4..12].copy_from_slice(&self.timestamp_us.to_le_bytes());
        buf[12..16].copy_from_slice(&self.sig_truncated.to_le_bytes());
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_canonical_event() {
        let original = ConsentEvent {
            state: 0x02,
            flags: FLAG_REPLAY_TOLERANT,
            manifest_id: 0xA5B4,
            timestamp_us: 0x0102_0304_0506_0708,
            sig_truncated: 0xDEAD_BEEF,
        };
        let bytes = original.to_bytes();
        assert_eq!(bytes.len(), 16);
        let decoded = ConsentEvent::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn rejects_undersize() {
        assert!(matches!(
            ConsentEvent::from_bytes(&[0u8; 15]),
            Err(ConsentError::WireFormatLength)
        ));
    }

    #[test]
    fn rejects_oversize() {
        assert!(matches!(
            ConsentEvent::from_bytes(&[0u8; 17]),
            Err(ConsentError::WireFormatLength)
        ));
    }

    #[test]
    fn rejects_reserved_flag_bit() {
        let mut buf = [0u8; 16];
        buf[0] = 0x01;
        buf[1] = 0x80;
        assert!(matches!(
            ConsentEvent::from_bytes(&buf),
            Err(ConsentError::ReservedFlagBit)
        ));
    }

    #[test]
    fn rejects_reserved_state_discriminant() {
        let mut buf = [0u8; 16];
        buf[0] = 0x00;
        assert!(matches!(
            ConsentEvent::from_bytes(&buf),
            Err(ConsentError::ReservedDiscriminant)
        ));
        buf[0] = 0xFF;
        assert!(matches!(
            ConsentEvent::from_bytes(&buf),
            Err(ConsentError::ReservedDiscriminant)
        ));
    }
}
