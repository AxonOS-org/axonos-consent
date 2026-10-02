// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! Authentication: the boundary between bytes and facts (SPEC §7).
//!
//! A frame becomes an [`Authenticated`] record only here, and only after its
//! signature verifies under the key of the party the record names. The type has
//! no public constructor, so code outside this crate cannot produce one, and the
//! machines accept nothing else. There is no path from a frame to a state change
//! that does not pass through a signature check.
//!
//! Verification is pluggable through [`SignatureVerifier`] so that a secure
//! element can do the arithmetic. The reference implementation,
//! [`Ed25519Strict`], is enabled by the default `ed25519` feature.

use crate::error::ConsentError;
use crate::wire::{ConsentRecord, Frame, Party, BODY_LEN, SIGNATURE_LEN};

/// Something that can verify a signature over a record.
///
/// Implementations must verify Ed25519 signatures as RFC 8032 defines them,
/// under the strict rules of SPEC §7.3: a non-canonical `S`, a non-canonical or
/// small-order `R`, and a small-order public key are all refused. Verification
/// handles only public data, so it has no constant-time obligation.
pub trait SignatureVerifier {
    /// True if, and only if, `signature` is a valid strict Ed25519 signature
    /// over `message` under `public_key`.
    fn verify(
        &self,
        public_key: &[u8; 32],
        message: &[u8; BODY_LEN],
        signature: &[u8; SIGNATURE_LEN],
    ) -> bool;

    /// True if `public_key` may serve as a trust anchor. Called once, when a
    /// machine is built. The default accepts every key; the reference verifier
    /// refuses keys that do not decode or that have small order.
    fn accepts_key(&self, public_key: &[u8; 32]) -> bool {
        let _ = public_key;
        true
    }
}

/// The reference verifier: Ed25519 from `ed25519-dalek`, strict mode.
#[cfg(feature = "ed25519")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Ed25519Strict;

#[cfg(feature = "ed25519")]
impl SignatureVerifier for Ed25519Strict {
    fn verify(
        &self,
        public_key: &[u8; 32],
        message: &[u8; BODY_LEN],
        signature: &[u8; SIGNATURE_LEN],
    ) -> bool {
        let Ok(key) = ed25519_dalek::VerifyingKey::from_bytes(public_key) else {
            return false;
        };
        let signature = ed25519_dalek::Signature::from_bytes(signature);
        key.verify_strict(message, &signature).is_ok()
    }

    fn accepts_key(&self, public_key: &[u8; 32]) -> bool {
        matches!(
            ed25519_dalek::VerifyingKey::from_bytes(public_key),
            Ok(key) if !key.is_weak()
        )
    }
}

/// A record whose signature has been verified, together with the party whose
/// key verified it. Only this crate can construct one.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Authenticated {
    record: ConsentRecord,
}

impl Authenticated {
    /// The verified record.
    pub const fn record(&self) -> &ConsentRecord {
        &self.record
    }
    /// The party whose key verified the signature.
    pub const fn party(&self) -> Party {
        self.record.party()
    }
}

/// Parse, decode and verify a frame. `guardian` is `None` for a single-party
/// machine, in which case a record naming the guardian is refused.
pub(crate) fn authenticate<V: SignatureVerifier + ?Sized>(
    frame: &[u8],
    patient: &[u8; 32],
    guardian: Option<&[u8; 32]>,
    verifier: &V,
) -> Result<Authenticated, ConsentError> {
    let frame = Frame::parse(frame)?;
    let record = ConsentRecord::decode(frame.body())?;
    let key = match record.party() {
        Party::Patient => patient,
        Party::Guardian => guardian.ok_or(ConsentError::SignatureInvalid)?,
    };
    if verifier.verify(key, frame.body(), frame.signature()) {
        Ok(Authenticated { record })
    } else {
        Err(ConsentError::SignatureInvalid)
    }
}
