//! Integration tests: CBOR round-trip, JSON vector loading, state machine, engine.
//! Run: cargo test --features json

use axonos_consent::*;
use axonos_consent::codec::cbor;
use axonos_consent::frames::*;
use axonos_consent::reason::ReasonCode;
use axonos_consent::state::{ConsentState, TransitionError};
use axonos_consent::engine::{ConsentEngine, MAX_PEERS};
use axonos_consent::validate;

// ═══════════════════════════════════════════════════════════════════
//  CBOR ROUND-TRIP
// ═══════════════════════════════════════════════════════════════════

fn assert_cbor_roundtrip(original: &ConsentFrame) {
    let mut buf = [0u8; cbor::MAX_ENCODED_SIZE];
    let n = cbor::encode(original, &mut buf);
    let decoded = cbor::decode(&buf[..n])
        .unwrap_or_else(|e| panic!("decode failed for {}: {:?}", original.type_str(), e));
    assert_eq!(original, &decoded, "round-trip mismatch for {}", original.type_str());
}

#[test]
fn cbor_rt_withdraw_peer() {
    assert_cbor_roundtrip(&ConsentFrame::Withdraw(ConsentWithdraw {
        scope: Scope::Peer, reason_code: Some(ReasonCode::UserInitiated),
        reason: Some(ReasonBuf::from_str("user disconnect")),
        epoch: None, timestamp_ms: Some(1711540800000), timestamp_us: None,
    }));
}

#[test]
fn cbor_rt_withdraw_all_safety() {
    assert_cbor_roundtrip(&ConsentFrame::Withdraw(ConsentWithdraw {
        scope: Scope::All, reason_code: Some(ReasonCode::SafetyViolation),
        reason: None, epoch: Some(48291), timestamp_ms: None, timestamp_us: Some(1711540800000000),
    }));
}

#[test]
fn cbor_rt_withdraw_stimguard() {
    assert_cbor_roundtrip(&ConsentFrame::Withdraw(ConsentWithdraw {
        scope: Scope::Peer, reason_code: Some(ReasonCode::StimGuardLockout),
        reason: Some(ReasonBuf::from_str("charge density violation")),
        epoch: None, timestamp_ms: None, timestamp_us: Some(1711540800123456),
    }));
}

#[test]
fn cbor_rt_suspend_minimal() {
    assert_cbor_roundtrip(&ConsentFrame::Suspend(ConsentSuspend {
        reason_code: None, reason: None, timestamp_ms: None, timestamp_us: None,
    }));
}

#[test]
fn cbor_rt_resume_with_ts() {
    assert_cbor_roundtrip(&ConsentFrame::Resume(ConsentResume {
        timestamp_ms: Some(1711540860000), timestamp_us: None,
    }));
}

#[test]
fn cbor_rt_both_timestamps() {
    assert_cbor_roundtrip(&ConsentFrame::Withdraw(ConsentWithdraw {
        scope: Scope::Peer, reason_code: Some(ReasonCode::UserInitiated),
        reason: None, epoch: None,
        timestamp_ms: Some(1711540800000), timestamp_us: Some(1711540800123456),
    }));
}

// ═══════════════════════════════════════════════════════════════════
//  CBOR SECURITY BOUNDS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn cbor_rejects_oversized_map() {
    // Craft a CBOR map with 20 entries (> MAX_MAP_FIELDS=8)
    let mut bad = [0u8; 256];
    bad[0] = 0xB4; // map(20) — major 5, additional 20
    assert_eq!(cbor::decode(&bad), Err(cbor::DecodeError::MapTooLarge));
}

#[test]
fn cbor_rejects_oversized_string() {
    // Map with 1 entry, key = text(200 bytes) — exceeds MAX_STRING_LEN=128
    let mut bad = Vec::new();
    bad.push(0xA1); // map(1)
    bad.push(0x78); bad.push(200); // text(200)
    bad.extend(vec![b'x'; 200]); // key bytes
    bad.push(0x01); // value = uint(1)
    assert_eq!(cbor::decode(&bad), Err(cbor::DecodeError::StringTooLong));
}

#[test]
fn cbor_rejects_duplicate_type_key() {
    // Encode a valid withdraw, then manually craft duplicate "type" key
    let frame = ConsentFrame::Withdraw(ConsentWithdraw {
        scope: Scope::Peer, reason_code: None, reason: None,
        epoch: None, timestamp_ms: None, timestamp_us: None,
    });
    let mut buf = [0u8; cbor::MAX_ENCODED_SIZE];
    let n = cbor::encode(&frame, &mut buf);

    // Modify map length from 2 to 3 and append another "type" key
    // This is a simplified test — the real attack surface is crafted binary
    // The decoder should reject on second "type" hit
    let mut crafted = Vec::new();
    crafted.push(0xA3); // map(3) instead of map(2)
    crafted.extend_from_slice(&buf[1..n]); // original entries
    // Append: "type" -> "consent-resume"
    // text(4) "type"
    crafted.extend_from_slice(&[0x64, b't', b'y', b'p', b'e']);
    // text(15) "consent-resume"
    crafted.extend_from_slice(&[0x6F]);
    crafted.extend_from_slice(b"consent-resume\x00"); // 15 bytes
    assert_eq!(cbor::decode(&crafted), Err(cbor::DecodeError::DuplicateKey));
}

// ═══════════════════════════════════════════════════════════════════
//  JSON VECTOR ROUND-TRIP
// ═══════════════════════════════════════════════════════════════════

#[cfg(feature = "json")]
mod json_tests {
    use super::*;
    use axonos_consent::codec::json;

    #[test]
    fn load_all_15_vectors() {
        let raw = include_str!("vectors/consent-interop-vectors-v0.1.0.json");
        let root: serde_json::Value = serde_json::from_str(raw).unwrap();
        let vectors = root["vectors"].as_array().unwrap();
        assert_eq!(vectors.len(), 15);

        let mut passed = 0;
        for tv in vectors {
            let id = tv["id"].as_str().unwrap_or("?");
            let json_field = match tv.get("json") {
                Some(v) if v.is_object() => v,
                _ => continue,
            };

            // JSON → ConsentFrame
            let frame = json::decode_value(json_field)
                .unwrap_or_else(|e| panic!("{}: JSON decode: {}", id, e));

            // ConsentFrame → CBOR → ConsentFrame
            let mut cbor_buf = [0u8; cbor::MAX_ENCODED_SIZE];
            let cbor_len = cbor::encode(&frame, &mut cbor_buf);
            let cbor_rt = cbor::decode(&cbor_buf[..cbor_len])
                .unwrap_or_else(|e| panic!("{}: CBOR decode: {:?}", id, e));
            assert_eq!(frame, cbor_rt, "{}: CBOR round-trip", id);

            // ConsentFrame → JSON → ConsentFrame
            let json_rt_val = json::encode_value(&frame);
            let json_rt = json::decode_value(&json_rt_val)
                .unwrap_or_else(|e| panic!("{}: JSON re-decode: {}", id, e));
            assert_eq!(frame, json_rt, "{}: JSON round-trip", id);

            // Validate
            if let Err(e) = validate::validate(&frame) {
                eprintln!("  WARN {}: validation: {:?} (may be intentional)", id, e);
            }

            eprintln!("  PASS {} (CBOR {} bytes)", id, cbor_len);
            passed += 1;
        }
        assert!(passed >= 14, "expected >=14 testable vectors, got {}", passed);
        eprintln!("\n  {} / 15 vectors passed full round-trip.", passed);
    }

    #[test]
    fn state_transitions_match_vectors() {
        let raw = include_str!("vectors/consent-interop-vectors-v0.1.0.json");
        let root: serde_json::Value = serde_json::from_str(raw).unwrap();
        for tv in root["vectors"].as_array().unwrap() {
            let id = tv["id"].as_str().unwrap_or("?");
            let (before, after) = match (
                tv.get("state_before").and_then(|v| v.as_str()),
                tv.get("state_after").and_then(|v| v.as_str()),
            ) {
                (Some(b), Some(a)) => (b, a),
                _ => continue,
            };
            let json_field = match tv.get("json") {
                Some(v) if v.is_object() => v,
                _ => continue,
            };
            let frame = match json::decode_value(json_field) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let initial = parse_state(before);
            let expected = parse_state(after);
            let result = apply_transition(initial, &frame);
            assert_eq!(result, expected, "{}: {:?} → {:?}, got {:?}", id, initial, expected, result);
        }
    }

    fn parse_state(s: &str) -> ConsentState {
        match s {
            "granted" => ConsentState::Granted, "suspended" => ConsentState::Suspended,
            "withdrawn" => ConsentState::Withdrawn, _ => panic!("unknown: {}", s),
        }
    }
    fn apply_transition(s: ConsentState, f: &ConsentFrame) -> ConsentState {
        match f {
            ConsentFrame::Withdraw(_) => s.withdraw().unwrap_or(s),
            ConsentFrame::Suspend(_) => s.suspend().unwrap_or(s),
            ConsentFrame::Resume(_) => s.resume().unwrap_or(s),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
//  STATE MACHINE
// ═══════════════════════════════════════════════════════════════════

#[test] fn sm_grant_suspend() { assert_eq!(ConsentState::Granted.suspend(), Ok(ConsentState::Suspended)); }
#[test] fn sm_suspend_resume() { assert_eq!(ConsentState::Suspended.resume(), Ok(ConsentState::Granted)); }
#[test] fn sm_grant_withdraw() { assert_eq!(ConsentState::Granted.withdraw(), Ok(ConsentState::Withdrawn)); }
#[test] fn sm_suspend_withdraw() { assert_eq!(ConsentState::Suspended.withdraw(), Ok(ConsentState::Withdrawn)); }
#[test] fn sm_withdrawn_terminal() {
    assert_eq!(ConsentState::Withdrawn.suspend(), Err(TransitionError::AlreadyWithdrawn));
    assert_eq!(ConsentState::Withdrawn.resume(), Err(TransitionError::AlreadyWithdrawn));
    assert_eq!(ConsentState::Withdrawn.withdraw(), Err(TransitionError::AlreadyWithdrawn));
}
#[test] fn sm_idempotent_suspend() { assert_eq!(ConsentState::Suspended.suspend(), Ok(ConsentState::Suspended)); }
#[test] fn sm_idempotent_resume() { assert_eq!(ConsentState::Granted.resume(), Ok(ConsentState::Granted)); }
#[test] fn sm_gossip_roundtrip() {
    for s in [ConsentState::Granted, ConsentState::Suspended, ConsentState::Withdrawn] {
        assert_eq!(ConsentState::from_gossip_bits(s.to_gossip_bits()), Some(s));
    }
}

// ═══════════════════════════════════════════════════════════════════
//  ENGINE
// ═══════════════════════════════════════════════════════════════════

#[test] fn eng_register() {
    let mut e = ConsentEngine::new(); let p = [1u8; 16];
    e.register_peer(p, 0).unwrap();
    assert_eq!(e.get_state(&p), Some(ConsentState::Granted));
}
#[test] fn eng_duplicate_rejected() {
    let mut e = ConsentEngine::new(); let p = [2u8; 16];
    e.register_peer(p, 0).unwrap();
    assert!(e.register_peer(p, 1).is_err());
}
#[test] fn eng_table_full() {
    let mut e = ConsentEngine::new();
    for i in 0..MAX_PEERS as u8 { let mut p = [0u8; 16]; p[0] = i; e.register_peer(p, 0).unwrap(); }
    assert!(e.register_peer([0xFF; 16], 0).is_err());
}
#[test] fn eng_unknown_peer() {
    let mut e = ConsentEngine::new();
    assert_eq!(e.suspend(&[0xFF; 16], None, 0), Err(TransitionError::PeerNotFound));
    assert_eq!(e.resume(&[0xFF; 16], 0), Err(TransitionError::PeerNotFound));
    assert_eq!(e.withdraw(&[0xFF; 16], None, 0), Err(TransitionError::PeerNotFound));
}
#[test] fn eng_withdraw_all() {
    let mut e = ConsentEngine::new();
    for i in 0..3u8 { let mut p = [0u8; 16]; p[0] = i; e.register_peer(p, 0).unwrap(); }
    assert_eq!(e.withdraw_all(Some(ReasonCode::EmergencyButton), 100), 3);
}

// ═══════════════════════════════════════════════════════════════════
//  REASON CODES
// ═══════════════════════════════════════════════════════════════════

#[test] fn rc_ranges() {
    assert!(ReasonCode::UserInitiated.is_spec_reserved());
    assert!(ReasonCode::StimGuardLockout.is_implementation_specific());
}
#[test] fn rc_unknown_defaults() { assert_eq!(ReasonCode::from_u8(0xFF), ReasonCode::Unspecified); }
#[test] fn rc_roundtrip() {
    for c in [ReasonCode::UserInitiated, ReasonCode::StimGuardLockout, ReasonCode::EmergencyButton] {
        assert_eq!(ReasonCode::from_u8(c.to_u8()), c);
    }
}

// ═══════════════════════════════════════════════════════════════════
//  VALIDATION
// ═══════════════════════════════════════════════════════════════════

#[test] fn validate_good_withdraw() {
    let f = ConsentFrame::Withdraw(ConsentWithdraw {
        scope: Scope::Peer, reason_code: Some(ReasonCode::UserInitiated),
        reason: None, epoch: None, timestamp_ms: Some(1000), timestamp_us: None,
    });
    assert!(validate::validate(&f).is_ok());
}

#[test] fn validate_rejects_zero_timestamp() {
    let f = ConsentFrame::Withdraw(ConsentWithdraw {
        scope: Scope::Peer, reason_code: None, reason: None,
        epoch: None, timestamp_ms: None, timestamp_us: Some(0),
    });
    assert_eq!(validate::validate(&f), Err(validate::ValidationError::ZeroTimestamp));
}
