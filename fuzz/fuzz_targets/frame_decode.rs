// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! Wire format (SPEC §6): parsing and decoding are total, and every accepted
//! record re-encodes to exactly the bytes it came from.

#![no_main]

use axonos_consent::wire::{ConsentRecord, Frame};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(frame) = Frame::parse(data) {
        if let Ok(record) = ConsentRecord::decode(frame.body()) {
            assert_eq!(&record.encode(), frame.body());
        }
    }
});
