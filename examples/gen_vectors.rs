// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! Generate, or check, the conformance vectors in `vectors/`.
//!
//! ```text
//! cargo run --example gen_vectors --features std            # write
//! cargo run --example gen_vectors --features std -- --check # verify, byte for byte
//! ```
//!
//! Every frame is signed with the RFC 8032 §7.1 TEST 1 key (the patient) or
//! TEST 2 key (another signer). Ed25519 is deterministic, so the output is the
//! same on every machine, and anyone can reproduce it without this code.

use std::fmt::Write as _;
use std::path::Path;
use std::process::ExitCode;

use axonos_consent::wire::{ConsentRecord, BODY_LEN, FLAG_GUARDIAN, FRAME_LEN};
use axonos_consent::{ConsentError, ConsentMachine, ConsentState, Ed25519Strict, Persisted};
use ed25519_dalek::{Signer, SigningKey};

const MANIFEST: u16 = 0x0007;
const TEST_1: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const TEST_2: &str = "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb";

fn key(hex: &str) -> SigningKey {
    let mut b = [0u8; 32];
    for (i, x) in b.iter_mut().enumerate() {
        *x = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).expect("hex");
    }
    SigningKey::from_bytes(&b)
}

fn frame(signer: &SigningKey, body: [u8; BODY_LEN]) -> Vec<u8> {
    let mut f = body.to_vec();
    f.extend_from_slice(&signer.sign(&body).to_bytes());
    f
}

struct Vector {
    name: &'static str,
    initial: ConsentState,
    sequence: u64,
    frame: Vec<u8>,
    about: &'static str,
}

fn vectors() -> Vec<Vector> {
    let p = key(TEST_1);
    let o = key(TEST_2);
    let rec = |s, seq| ConsentRecord::new(s, MANIFEST, seq, 1_000_000 * seq).encode();
    use ConsentState::{Granted as G, Suspended as S, Withdrawn as W};
    let raw = |mutate: &dyn Fn(&mut [u8; BODY_LEN])| {
        let mut b = rec(G, 1);
        mutate(&mut b);
        frame(&p, b)
    };
    let mut forged = frame(&p, rec(S, 1));
    forged[BODY_LEN] ^= 0x01;
    let mut malleable = frame(&p, rec(S, 1));
    const L: [u8; 32] = [
        0xED, 0xD3, 0xF5, 0x5C, 0x1A, 0x63, 0x12, 0x58, 0xD6, 0x9C, 0xF7, 0xA2, 0xDE, 0xF9, 0xDE,
        0x14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
    ];
    let mut carry = 0u16;
    for (i, l) in L.iter().enumerate() {
        let sum = u16::from(malleable[BODY_LEN + 32 + i]) + u16::from(*l) + carry;
        malleable[BODY_LEN + 32 + i] = sum as u8;
        carry = sum >> 8;
    }
    let guardian_body = ConsentRecord::new(S, MANIFEST, 1, 1_000_000)
        .by_guardian()
        .encode();
    let mut short = frame(&p, rec(S, 1));
    short.pop();
    let mut long = frame(&p, rec(S, 1));
    long.push(0);
    vec![
        Vector {
            name: "granted-to-suspended",
            initial: G,
            sequence: 0,
            frame: frame(&p, rec(S, 1)),
            about: "pause",
        },
        Vector {
            name: "suspended-to-granted",
            initial: S,
            sequence: 1,
            frame: frame(&p, rec(G, 2)),
            about: "resume",
        },
        Vector {
            name: "granted-to-withdrawn",
            initial: G,
            sequence: 2,
            frame: frame(&p, rec(W, 3)),
            about: "withdraw while granted",
        },
        Vector {
            name: "suspended-to-withdrawn",
            initial: S,
            sequence: 3,
            frame: frame(&p, rec(W, 4)),
            about: "withdraw while suspended",
        },
        Vector {
            name: "idempotent-granted",
            initial: G,
            sequence: 4,
            frame: frame(&p, rec(G, 5)),
            about: "re-assert the current state",
        },
        Vector {
            name: "withdrawn-to-granted-refused",
            initial: W,
            sequence: 5,
            frame: frame(&p, rec(G, 6)),
            about: "nothing leaves Withdrawn",
        },
        Vector {
            name: "withdrawn-to-suspended-refused",
            initial: W,
            sequence: 6,
            frame: frame(&p, rec(S, 7)),
            about: "nothing leaves Withdrawn",
        },
        Vector {
            name: "reserved-discriminant-refused",
            initial: G,
            sequence: 0,
            frame: raw(&|b| b[4] = 0x04),
            about: "state byte 0x04",
        },
        Vector {
            name: "reserved-flag-bit-refused",
            initial: G,
            sequence: 0,
            frame: raw(&|b| b[5] = 0x80),
            about: "flag bit 7",
        },
        Vector {
            name: "undersize-frame-refused",
            initial: G,
            sequence: 0,
            frame: short,
            about: "95 bytes",
        },
        Vector {
            name: "oversize-frame-refused",
            initial: G,
            sequence: 0,
            frame: long,
            about: "97 bytes",
        },
        Vector {
            name: "wrong-manifest-refused",
            initial: G,
            sequence: 0,
            frame: frame(&p, ConsentRecord::new(S, MANIFEST + 1, 1, 0).encode()),
            about: "manifest 0x0008",
        },
        Vector {
            name: "forged-signature-refused",
            initial: G,
            sequence: 0,
            frame: forged,
            about: "one bit of R flipped",
        },
        Vector {
            name: "wrong-key-refused",
            initial: G,
            sequence: 0,
            frame: frame(&o, rec(S, 1)),
            about: "signed with the RFC 8032 TEST 2 key",
        },
        Vector {
            name: "replayed-sequence-refused",
            initial: G,
            sequence: 5,
            frame: frame(&p, rec(S, 5)),
            about: "sequence 5 after 5",
        },
        Vector {
            name: "terminal-flag-mismatch-refused",
            initial: G,
            sequence: 0,
            frame: raw(&|b| b[4] = 0x03),
            about: "Withdrawn without the terminal flag",
        },
        Vector {
            name: "bad-magic-refused",
            initial: G,
            sequence: 0,
            frame: raw(&|b| b[3] = b'1'),
            about: "AXC1",
        },
        Vector {
            name: "reserved-field-nonzero-refused",
            initial: G,
            sequence: 0,
            frame: raw(&|b| b[31] = 1),
            about: "byte 31 set",
        },
        Vector {
            name: "non-canonical-signature-refused",
            initial: G,
            sequence: 0,
            frame: malleable,
            about: "S + L, RFC 8032 malleability",
        },
        Vector {
            name: "guardian-on-single-party-refused",
            initial: G,
            sequence: 0,
            frame: frame(&o, guardian_body),
            about: "guardian flag, no guardian key",
        },
    ]
}

fn state_name(s: ConsentState) -> &'static str {
    match s {
        ConsentState::Granted => "Granted",
        ConsentState::Suspended => "Suspended",
        ConsentState::Withdrawn => "Withdrawn",
    }
}

fn expected(v: &Vector, n: usize) -> String {
    let p = key(TEST_1);
    let public = p.verifying_key().to_bytes();
    let mut m = ConsentMachine::restore(
        MANIFEST,
        public,
        Ed25519Strict,
        Persisted {
            state: v.initial,
            patient_sequence: v.sequence,
            guardian_sequence: 0,
        },
    )
    .expect("RFC 8032 key");
    let outcome: Result<ConsentState, ConsentError> = m.handle(&v.frame);
    let hex: String = public.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    });
    let (result, after, error, code) = match outcome {
        Ok(s) => (
            "accept",
            state_name(s),
            "null".to_string(),
            "null".to_string(),
        ),
        Err(e) => (
            "refuse",
            state_name(m.state()),
            format!("\"{e:?}\""),
            format!("\"0x{:02X}\"", e.to_abi_code()),
        ),
    };
    let _ = FLAG_GUARDIAN;
    format!(
        "{{\n  \"vector\": \"vector-{n:02}-{}\",\n  \"about\": \"{}\",\n  \"profile\": \"single-party\",\n  \"wire_version\": 2,\n  \"manifest_id\": {MANIFEST},\n  \"trusted_key\": \"{hex}\",\n  \"initial_state\": \"{}\",\n  \"initial_sequence\": {},\n  \"frame_len\": {},\n  \"result\": \"{result}\",\n  \"state_after\": \"{after}\",\n  \"error\": {error},\n  \"abi_code\": {code}\n}}\n",
        v.name,
        v.about,
        state_name(v.initial),
        v.sequence,
        v.frame.len(),
    )
}

fn main() -> ExitCode {
    let check = std::env::args().any(|a| a == "--check");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("vectors");
    let mut drift = 0;
    for (i, v) in vectors().iter().enumerate() {
        let n = i + 1;
        let stem = format!("vector-{n:02}-{}", v.name);
        let files = [
            (dir.join(format!("{stem}.bin")), v.frame.clone()),
            (
                dir.join(format!("{stem}.expected.json")),
                expected(v, n).into_bytes(),
            ),
        ];
        for (path, bytes) in files {
            if check {
                if std::fs::read(&path).ok().as_deref() != Some(&bytes[..]) {
                    eprintln!("drift: {}", path.display());
                    drift += 1;
                }
            } else {
                std::fs::write(&path, &bytes).expect("write vector");
            }
        }
        assert!(v.frame.len() == FRAME_LEN || v.name.contains("size"));
    }
    if drift > 0 {
        eprintln!("{drift} file(s) differ from the generator");
        return ExitCode::FAILURE;
    }
    println!(
        "{} vectors {}",
        vectors().len(),
        if check {
            "match the generator"
        } else {
            "written"
        }
    );
    ExitCode::SUCCESS
}
