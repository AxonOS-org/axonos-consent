// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! Forgery (SPEC §7): no frame may ever be admitted under a key whose secret
//! no one holds.
//!
//! The key is a curve point derived from SHA-512 of a fixed label and
//! multiplied by the cofactor; `tools/check.py` repeats the derivation on every
//! change. Its discrete logarithm is unknown to anyone, so no genuine signature
//! under it exists — in the seed corpus, in the conformance vectors, anywhere.
//! An admitted frame could therefore only be a forgery, whatever the fuzzer
//! starts from.

#![no_main]

use axonos_consent::{ConsentMachine, ConsentState, Ed25519Strict};
use libfuzzer_sys::fuzz_target;

/// SHA-512("axonos-consent/fuzz/auth_forgery: a public key whose secret no one
/// holds" ‖ 0u32), first 32 bytes, decompressed, times the cofactor 8.
const KEY_NO_ONE_HOLDS: [u8; 32] = [
    0xfa, 0x17, 0x7b, 0x04, 0x7e, 0xb2, 0x21, 0x8c, 0x9e, 0x5c, 0xec, 0x64, 0x30, 0x44, 0x48, 0x37,
    0x19, 0xb0, 0xbe, 0x8f, 0xd5, 0xc7, 0xe3, 0xe3, 0x9b, 0x89, 0x6c, 0xbc, 0x43, 0x5d, 0x38, 0x79,
];

fuzz_target!(|data: &[u8]| {
    let mut machine =
        ConsentMachine::new(7, KEY_NO_ONE_HOLDS, Ed25519Strict).expect("a prime-order point");
    assert!(machine.handle(data).is_err(), "forged frame admitted");
    assert_eq!(machine.state(), ConsentState::Granted);
});
