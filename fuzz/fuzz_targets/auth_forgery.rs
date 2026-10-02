// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! Forgery (SPEC §7): against the real verifier and the RFC 8032 TEST 1 public
//! key, no frame the fuzzer builds may ever be admitted. The seed corpus holds
//! only refused frames, so an admitted frame would be a forgery.

#![no_main]

use axonos_consent::{ConsentMachine, ConsentState, Ed25519Strict};
use libfuzzer_sys::fuzz_target;

const TEST_1_PUBLIC: [u8; 32] = [
    0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64, 0x07, 0x3a,
    0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68, 0xf7, 0x07, 0x51, 0x1a,
];

fuzz_target!(|data: &[u8]| {
    let mut machine = ConsentMachine::new(7, TEST_1_PUBLIC, Ed25519Strict).expect("RFC 8032 key");
    assert!(machine.handle(data).is_err(), "forged frame admitted");
    assert_eq!(machine.state(), ConsentState::Granted);
});
