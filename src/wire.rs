// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! The wire format, version 2: a 32-byte record and its 64-byte Ed25519
//! signature, 96 bytes in all (SPEC §6).
//!
//! ```text
//!  offset  size  field          meaning
//!       0     4  magic          "AXC2" — protocol and version; the domain separator
//!       4     1  state          0x01 Granted · 0x02 Suspended · 0x03 Withdrawn
//!       5     1  flags          bit 0 terminal · bit 1 from-secure-world · bit 3 guardian
//!       6     2  manifest_id    u16, little-endian
//!       8     8  sequence       u64, little-endian; strictly increasing per signer
//!      16     8  timestamp_us   u64, little-endian; the signer's clock, informational only
//!      24     8  reserved       zero
//!      32    64  signature      Ed25519 (RFC 8032) over bytes 0..32
//! ```
//!
//! Decoding is total and canonical: every 32-byte input is either refused with
//! a typed error or accepted, and an accepted record re-encodes to exactly the
//! bytes it came from. Both properties are proved by Kani (`src/proofs.rs`).

use crate::error::ConsentError;
use crate::state::ConsentState;

/// The first four bytes of every record: protocol `AXC`, wire version `2`.
pub const MAGIC: [u8; 4] = *b"AXC2";
/// The wire-format version this crate speaks.
pub const WIRE_VERSION: u8 = 2;
/// Length of the signed record.
pub const BODY_LEN: usize = 32;
/// Length of an Ed25519 signature.
pub const SIGNATURE_LEN: usize = 64;
/// Length of a frame: the record followed by its signature.
pub const FRAME_LEN: usize = BODY_LEN + SIGNATURE_LEN;

/// Flag bit 0: the state is `Withdrawn`. Must agree with the state byte.
pub const FLAG_TERMINAL: u8 = 1 << 0;
/// Flag bit 1: the event originated in a TrustZone-M Secure-World UI. Signed,
/// recorded, and carried into the audit log; it does not change admission.
pub const FLAG_FROM_SECURE_WORLD: u8 = 1 << 1;
/// Flag bit 3: the signer is the guardian, not the patient (SPEC §12.2). The
/// role is part of the signed record, so it cannot be claimed after signing.
pub const FLAG_GUARDIAN: u8 = 1 << 3;
/// Every defined flag bit. Bit 2 (replay-tolerant in wire v1) is retired and,
/// like bits 4–7, reserved.
pub const FLAGS_DEFINED_MASK: u8 = FLAG_TERMINAL | FLAG_FROM_SECURE_WORLD | FLAG_GUARDIAN;

/// Who signed a record.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum Party {
    /// The patient: the trusted-path key.
    Patient,
    /// The guardian: the second key of a dual-control deployment.
    Guardian,
}

/// A decoded consent record. Every value of this type satisfies the record
/// invariants of SPEC §6, because the only ways to obtain one are
/// [`ConsentRecord::decode`] and the builder, which uphold them.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct ConsentRecord {
    state: ConsentState,
    flags: u8,
    manifest_id: u16,
    sequence: u64,
    timestamp_us: u64,
}

impl ConsentRecord {
    /// A record from the patient, with the terminal flag set to match `state`.
    pub const fn new(
        state: ConsentState,
        manifest_id: u16,
        sequence: u64,
        timestamp_us: u64,
    ) -> Self {
        Self {
            state,
            flags: if state.is_terminal() {
                FLAG_TERMINAL
            } else {
                0
            },
            manifest_id,
            sequence,
            timestamp_us,
        }
    }

    /// The same record, signed by the guardian.
    pub const fn by_guardian(mut self) -> Self {
        self.flags |= FLAG_GUARDIAN;
        self
    }

    /// The same record, marked as originating in a Secure-World UI.
    pub const fn from_secure_world(mut self) -> Self {
        self.flags |= FLAG_FROM_SECURE_WORLD;
        self
    }

    /// Decode and validate a record. Checks run in the order SPEC §7.6 fixes:
    /// magic, state, flags, terminal flag, reserved bytes.
    pub fn decode(body: &[u8; BODY_LEN]) -> Result<Self, ConsentError> {
        // One integer comparison rather than an array comparison: the decode
        // path stays free of loops, at the machine level and in Kani's model.
        if u32::from_le_bytes([body[0], body[1], body[2], body[3]]) != u32::from_le_bytes(MAGIC) {
            return Err(ConsentError::BadMagic);
        }
        let state = ConsentState::from_wire(body[4])?;
        let flags = body[5];
        if flags & !FLAGS_DEFINED_MASK != 0 {
            return Err(ConsentError::ReservedFlagBit);
        }
        if (flags & FLAG_TERMINAL != 0) != state.is_terminal() {
            return Err(ConsentError::TerminalFlagMismatch);
        }
        if (body[24] | body[25] | body[26] | body[27] | body[28] | body[29] | body[30] | body[31])
            != 0
        {
            return Err(ConsentError::ReservedFieldNonZero);
        }
        Ok(Self {
            state,
            flags,
            manifest_id: u16::from_le_bytes([body[6], body[7]]),
            sequence: u64::from_le_bytes([
                body[8], body[9], body[10], body[11], body[12], body[13], body[14], body[15],
            ]),
            timestamp_us: u64::from_le_bytes([
                body[16], body[17], body[18], body[19], body[20], body[21], body[22], body[23],
            ]),
        })
    }

    /// Encode to the 32 bytes that are signed.
    pub fn encode(&self) -> [u8; BODY_LEN] {
        let mut b = [0u8; BODY_LEN];
        b[0..4].copy_from_slice(&MAGIC);
        b[4] = self.state.as_u8();
        b[5] = self.flags;
        b[6..8].copy_from_slice(&self.manifest_id.to_le_bytes());
        b[8..16].copy_from_slice(&self.sequence.to_le_bytes());
        b[16..24].copy_from_slice(&self.timestamp_us.to_le_bytes());
        b
    }

    /// The target state.
    pub const fn state(&self) -> ConsentState {
        self.state
    }
    /// The flags byte.
    pub const fn flags(&self) -> u8 {
        self.flags
    }
    /// The manifest installation this record is for.
    pub const fn manifest_id(&self) -> u16 {
        self.manifest_id
    }
    /// The signer's sequence number.
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
    /// The signer's clock at signing. Informational: never used for admission,
    /// ordering or co-authorisation windows (SPEC §6.1).
    pub const fn timestamp_us(&self) -> u64 {
        self.timestamp_us
    }
    /// Who signed it, as the record itself declares.
    pub const fn party(&self) -> Party {
        if self.flags & FLAG_GUARDIAN != 0 {
            Party::Guardian
        } else {
            Party::Patient
        }
    }
}

/// A frame split into its record and signature, not yet decoded or verified.
#[derive(Copy, Clone, Debug)]
pub struct Frame<'a> {
    body: &'a [u8; BODY_LEN],
    signature: &'a [u8; SIGNATURE_LEN],
}

impl<'a> Frame<'a> {
    /// Split a frame of exactly [`FRAME_LEN`] bytes.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ConsentError> {
        if bytes.len() != FRAME_LEN {
            return Err(ConsentError::WireFormatLength);
        }
        let (body, signature) = bytes.split_at(BODY_LEN);
        Ok(Self {
            body: body
                .try_into()
                .map_err(|_| ConsentError::WireFormatLength)?,
            signature: signature
                .try_into()
                .map_err(|_| ConsentError::WireFormatLength)?,
        })
    }

    /// The 32 signed bytes.
    pub const fn body(&self) -> &'a [u8; BODY_LEN] {
        self.body
    }

    /// The 64-byte signature.
    pub const fn signature(&self) -> &'a [u8; SIGNATURE_LEN] {
        self.signature
    }
}

/// Assemble a frame from a record and a signature over its encoding. For
/// signers and tests; the kernel only ever parses.
pub fn assemble(record: &ConsentRecord, signature: &[u8; SIGNATURE_LEN]) -> [u8; FRAME_LEN] {
    let mut frame = [0u8; FRAME_LEN];
    frame[..BODY_LEN].copy_from_slice(&record.encode());
    frame[BODY_LEN..].copy_from_slice(signature);
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(state: u8, flags: u8) -> [u8; BODY_LEN] {
        let mut b = ConsentRecord::new(ConsentState::Granted, 7, 1, 0).encode();
        b[4] = state;
        b[5] = flags;
        b
    }

    #[test]
    fn roundtrip_is_exact() {
        let r = ConsentRecord::new(ConsentState::Withdrawn, 0xA5B4, 0x0102_0304_0506_0708, 99)
            .by_guardian()
            .from_secure_world();
        assert_eq!(ConsentRecord::decode(&r.encode()), Ok(r));
        assert_eq!(r.party(), Party::Guardian);
        assert_eq!(
            r.flags(),
            FLAG_TERMINAL | FLAG_GUARDIAN | FLAG_FROM_SECURE_WORLD
        );
    }

    #[test]
    fn refusals_follow_the_normative_order() {
        let mut b = body(0x00, 0x80);
        b[0] = b'X';
        assert_eq!(ConsentRecord::decode(&b), Err(ConsentError::BadMagic));
        b[0] = b'A';
        assert_eq!(
            ConsentRecord::decode(&b),
            Err(ConsentError::ReservedDiscriminant)
        );
        b[4] = 0x01;
        assert_eq!(
            ConsentRecord::decode(&b),
            Err(ConsentError::ReservedFlagBit)
        );
        b[5] = FLAG_TERMINAL;
        assert_eq!(
            ConsentRecord::decode(&b),
            Err(ConsentError::TerminalFlagMismatch)
        );
        b[5] = 0;
        b[31] = 1;
        assert_eq!(
            ConsentRecord::decode(&b),
            Err(ConsentError::ReservedFieldNonZero)
        );
    }

    #[test]
    fn withdrawn_without_terminal_flag_is_refused() {
        assert_eq!(
            ConsentRecord::decode(&body(0x03, 0)),
            Err(ConsentError::TerminalFlagMismatch)
        );
    }

    #[test]
    fn retired_replay_tolerant_bit_is_reserved() {
        assert_eq!(
            ConsentRecord::decode(&body(0x01, 1 << 2)),
            Err(ConsentError::ReservedFlagBit)
        );
    }

    #[test]
    fn frames_must_be_exactly_96_bytes() {
        assert!(Frame::parse(&[0u8; FRAME_LEN]).is_ok());
        for len in [0, 16, BODY_LEN, FRAME_LEN - 1, FRAME_LEN + 1] {
            assert!(matches!(
                Frame::parse(&[0u8; 128][..len]),
                Err(ConsentError::WireFormatLength)
            ));
        }
    }
}
