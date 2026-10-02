// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! The conformance vectors are executable: every vector in `vectors/` is run
//! against the machine and must produce exactly its documented outcome.

#![cfg(feature = "ed25519")]

mod common;

use std::path::Path;

use axonos_consent::{ConsentMachine, ConsentState, Ed25519Strict, Persisted};

fn field<'a>(json: &'a str, name: &str) -> &'a str {
    let at = json.find(&format!("\"{name}\": ")).expect(name) + name.len() + 4;
    let rest = &json[at..];
    let end = rest.find(['\n', ',']).expect("end of field");
    rest[..end].trim_matches('"')
}

fn state(name: &str) -> ConsentState {
    match name {
        "Granted" => ConsentState::Granted,
        "Suspended" => ConsentState::Suspended,
        "Withdrawn" => ConsentState::Withdrawn,
        other => panic!("unknown state {other}"),
    }
}

#[test]
fn every_vector_produces_its_documented_outcome() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("vectors");
    let mut count = 0;
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("vectors/")
        .flatten()
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".expected.json") else {
            continue;
        };
        let json = std::fs::read_to_string(entry.path()).expect("json");
        let frame = std::fs::read(dir.join(format!("{stem}.bin"))).expect("bin");
        assert_eq!(
            field(&json, "frame_len").parse::<usize>().unwrap(),
            frame.len(),
            "{stem}"
        );
        let mut machine = ConsentMachine::restore(
            field(&json, "manifest_id").parse().expect("manifest"),
            common::hex32(field(&json, "trusted_key")),
            Ed25519Strict,
            Persisted {
                state: state(field(&json, "initial_state")),
                patient_sequence: field(&json, "initial_sequence").parse().expect("sequence"),
                guardian_sequence: 0,
            },
        )
        .expect("vector key");
        let outcome = machine.handle(&frame);
        match field(&json, "result") {
            "accept" => assert_eq!(outcome, Ok(state(field(&json, "state_after"))), "{stem}"),
            "refuse" => {
                let error = outcome.expect_err(stem);
                assert_eq!(format!("{error:?}"), field(&json, "error"), "{stem}");
                assert_eq!(
                    format!("0x{:02X}", error.to_abi_code()),
                    field(&json, "abi_code"),
                    "{stem}"
                );
                assert_eq!(
                    machine.state(),
                    state(field(&json, "state_after")),
                    "{stem}"
                );
            }
            other => panic!("{stem}: unknown result {other}"),
        }
        count += 1;
    }
    assert_eq!(count, 20, "the vector set has twenty members");
}
