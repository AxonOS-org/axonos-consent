// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! Shared fixtures. The two keys are the RFC 8032 §7.1 TEST 1 and TEST 2 keys,
//! so anyone can reproduce every signature in this repository.

#![allow(dead_code)]

use axonos_consent::wire::{assemble, ConsentRecord, BODY_LEN, FRAME_LEN};
use ed25519_dalek::{Signer, SigningKey};

pub const MANIFEST: u16 = 0x0007;
pub const PATIENT_SECRET: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
pub const PATIENT_PUBLIC: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
pub const GUARDIAN_SECRET: &str =
    "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb";
pub const GUARDIAN_PUBLIC: &str =
    "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";

pub fn hex32(s: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("hex");
    }
    out
}

pub fn patient() -> SigningKey {
    SigningKey::from_bytes(&hex32(PATIENT_SECRET))
}

pub fn guardian() -> SigningKey {
    SigningKey::from_bytes(&hex32(GUARDIAN_SECRET))
}

pub fn public(key: &SigningKey) -> [u8; 32] {
    key.verifying_key().to_bytes()
}

pub fn sign(key: &SigningKey, record: ConsentRecord) -> [u8; FRAME_LEN] {
    assemble(&record, &key.sign(&record.encode()).to_bytes())
}

/// Sign arbitrary 32 bytes, valid record or not.
pub fn sign_raw(key: &SigningKey, body: [u8; BODY_LEN]) -> [u8; FRAME_LEN] {
    let mut frame = [0u8; FRAME_LEN];
    frame[..BODY_LEN].copy_from_slice(&body);
    frame[BODY_LEN..].copy_from_slice(&key.sign(&body).to_bytes());
    frame
}
