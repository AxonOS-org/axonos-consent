//! Integration tests for axonos-consent.
//!
//! Loads all 15 interop test vectors from consent-interop-vectors-v0.1.0.json,
//! validates JSON→ConsentFrame→CBOR→ConsentFrame round-trip,
//! and verifies state machine transitions match expected outcomes.
//!
//! Run with: cargo test --features json

extern crate alloc;

use axonos_consent::*;
use axonos_consent::codec::{cbor, json};
use axonos_consent::frames::*;
use axonos_consent::reason::ReasonCode;
use axonos_consent::state::{ConsentState, TransitionError};
use axonos_consent::engine::{ConsentEngine, MAX_PEERS};

// ═══════════════════════════════════════════════════════════════════
//  CBOR ROUND-TRIP TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn cbor_roundtrip_withdraw_peer_user_initiated() {
    let frame = ConsentFrame::Withdraw(ConsentWithdraw {
        scope: Scope::Peer,
        reason_code: Some(ReasonCode::UserInitiated),
        reason: Some("user requested disconnect".into()),
        epoch: None,
        timestamp_ms: Some(1711540800000),
        timestamp_us: None,
    });
    assert_cbor_roundtrip(&frame);
}

#[test]
fn cbor_roundtrip_withdraw_all_safety_violation() {
    let frame = ConsentFrame::Withdraw(ConsentWithdraw {
        scope: Scope::All,
        reason_code: Some(ReasonCode::SafetyViolation),
        reason: None,
        epoch: Some(48291),
        timestamp_ms: None,
        timestamp_us: Some(1711540800000000),
    });
    assert_cbor_roundtrip(&frame);
}

#[test]
fn cbor_roundtrip_withdraw_stimguard_lockout() {
    let frame = ConsentFrame::Withdraw(ConsentWithdraw {
        scope: Scope::Peer,
        reason_code: Some(ReasonCode::StimGuardLockout),
        reason: Some("stimguard lockout: repeated charge density violations".into()),
        epoch: None,
        timestamp_ms: None,
        timestamp_us: Some(1711540800123456),
    });
    assert_cbor_roundtrip(&frame);
}

#[test]
fn cbor_roundtrip_suspend_with_reason() {
    let frame = ConsentFrame::Suspend(ConsentSuspend {
        reason_code: Some(ReasonCode::UserInitiated),
        reason: Some("user entering focus mode".into()),
        timestamp_ms: Some(1711540800000),
        timestamp_us: None,
    });
    assert_cbor_roundtrip(&frame);
}

#[test]
fn cbor_roundtrip_suspend_minimal() {
    let frame = ConsentFrame::Suspend(ConsentSuspend {
        reason_code: None,
        reason: None,
        timestamp_ms: None,
        timestamp_us: None,
    });
    assert_cbor_roundtrip(&frame);
}

#[test]
fn cbor_roundtrip_resume_with_timestamp() {
    let frame = ConsentFrame::Resume(ConsentResume {
        timestamp_ms: Some(1711540860000),
        timestamp_us: None,
    });
    assert_cbor_roundtrip(&frame);
}

#[test]
fn cbor_roundtrip_resume_minimal() {
    let frame = ConsentFrame::Resume(ConsentResume {
        timestamp_ms: None,
        timestamp_us: None,
    });
    assert_cbor_roundtrip(&frame);
}

#[test]
fn cbor_roundtrip_withdraw_both_timestamps() {
    // TV-013: timestamp_us takes precedence, but both are preserved in round-trip
    let frame = ConsentFrame::Withdraw(ConsentWithdraw {
        scope: Scope::Peer,
        reason_code: Some(ReasonCode::UserInitiated),
        reason: None,
        epoch: None,
        timestamp_ms: Some(1711540800000),
        timestamp_us: Some(1711540800123456),
    });
    assert_cbor_roundtrip(&frame);
}

#[test]
fn cbor_roundtrip_withdraw_hardware_fault() {
    let frame = ConsentFrame::Withdraw(ConsentWithdraw {
        scope: Scope::Peer,
        reason_code: Some(ReasonCode::HardwareFault),
        reason: None,
        epoch: None,
        timestamp_ms: None,
        timestamp_us: Some(1711540800999999),
    });
    assert_cbor_roundtrip(&frame);
}

#[test]
fn cbor_roundtrip_withdraw_emergency_button() {
    // TV-015: reasonCode 0x12
    let frame = ConsentFrame::Withdraw(ConsentWithdraw {
        scope: Scope::All,
        reason_code: Some(ReasonCode::EmergencyButton),
        reason: Some("physical emergency button".into()),
        epoch: None,
        timestamp_ms: None,
        timestamp_us: Some(1711540800000001),
    });
    assert_cbor_roundtrip(&frame);
}

fn assert_cbor_roundtrip(original: &ConsentFrame) {
    let encoded = cbor::encode(original);
    let decoded = cbor::decode(&encoded)
        .unwrap_or_else(|e| panic!("CBOR decode failed for {:?}: {:?}", original.type_str(), e));
    assert_eq!(original, &decoded,
        "CBOR round-trip mismatch for {}.\nOriginal: {:?}\nDecoded:  {:?}\nCBOR hex: {}",
        original.type_str(), original, decoded, hex::encode(&encoded));
}

// ═══════════════════════════════════════════════════════════════════
//  JSON VECTOR LOADING TESTS
// ═══════════════════════════════════════════════════════════════════

#[cfg(feature = "json")]
mod json_vectors {
    use super::*;

    /// Load all 15 test vectors from the JSON file and validate round-trip.
    #[test]
    fn load_all_vectors_and_roundtrip() {
        let vectors_str = include_str!("vectors/consent-interop-vectors-v0.1.0.json");
        let root: serde_json::Value = serde_json::from_str(vectors_str)
            .expect("Failed to parse test vectors JSON");

        let vectors = root["vectors"].as_array()
            .expect("Expected 'vectors' array");

        assert_eq!(vectors.len(), 15, "Expected 15 test vectors, got {}", vectors.len());

        let mut tested = 0;

        for tv in vectors {
            let id = tv["id"].as_str().unwrap_or("unknown");
            let name = tv["name"].as_str().unwrap_or("unknown");

            // Some vectors (TV-014 gossip encoding) don't have a "json" field
            let json_field = match tv.get("json") {
                Some(v) if v.is_object() => v,
                _ => {
                    eprintln!("  SKIP {}: {} (no json field)", id, name);
                    continue;
                }
            };

            // Step 1: JSON → ConsentFrame
            let frame = json::decode_value(json_field)
                .unwrap_or_else(|e| panic!("{} ({}): JSON decode failed: {}", id, name, e));

            // Step 2: ConsentFrame → CBOR → ConsentFrame (round-trip)
            let cbor_bytes = cbor::encode(&frame);
            let cbor_decoded = cbor::decode(&cbor_bytes)
                .unwrap_or_else(|e| panic!("{} ({}): CBOR decode failed: {:?}", id, name, e));
            assert_eq!(frame, cbor_decoded,
                "{} ({}): CBOR round-trip mismatch", id, name);

            // Step 3: ConsentFrame → JSON → ConsentFrame (round-trip)
            let json_reencoded = json::encode_value(&frame);
            let json_redecoded = json::decode_value(&json_reencoded)
                .unwrap_or_else(|e| panic!("{} ({}): JSON re-decode failed: {}", id, name, e));
            assert_eq!(frame, json_redecoded,
                "{} ({}): JSON round-trip mismatch", id, name);

            // Step 4: Verify frame type string
            let expected_type = json_field["type"].as_str().unwrap();
            assert_eq!(frame.type_str(), expected_type,
                "{} ({}): frame type mismatch", id, name);

            eprintln!("  PASS {}: {} (CBOR {} bytes)", id, name, cbor_bytes.len());
            tested += 1;
        }

        assert!(tested >= 14, "Expected at least 14 testable vectors, only tested {}", tested);
        eprintln!("\n  All {} vectors passed round-trip.", tested);
    }

    /// Verify specific state transitions from test vectors.
    #[test]
    fn verify_state_transitions() {
        let vectors_str = include_str!("vectors/consent-interop-vectors-v0.1.0.json");
        let root: serde_json::Value = serde_json::from_str(vectors_str)
            .expect("Failed to parse test vectors JSON");

        let vectors = root["vectors"].as_array().unwrap();

        for tv in vectors {
            let id = tv["id"].as_str().unwrap_or("unknown");
            let state_before = tv.get("state_before").and_then(|v| v.as_str());
            let state_after = tv.get("state_after").and_then(|v| v.as_str());

            if let (Some(before), Some(after)) = (state_before, state_after) {
                let initial = parse_state(before);
                let expected = parse_state(after);

                let json_field = match tv.get("json") {
                    Some(v) if v.is_object() => v,
                    _ => continue,
                };

                let frame = match json::decode_value(json_field) {
                    Ok(f) => f,
                    Err(_) => continue,
                };

                let result = apply_transition(initial, &frame);
                assert_eq!(result, expected,
                    "{}: state transition mismatch. Before: {:?}, Expected after: {:?}, Got: {:?}",
                    id, initial, expected, result);

                eprintln!("  PASS {}: {:?} → {:?}", id, initial, expected);
            }
        }
    }

    fn parse_state(s: &str) -> ConsentState {
        match s {
            "granted" => ConsentState::Granted,
            "suspended" => ConsentState::Suspended,
            "withdrawn" => ConsentState::Withdrawn,
            _ => panic!("Unknown state: {}", s),
        }
    }

    fn apply_transition(state: ConsentState, frame: &ConsentFrame) -> ConsentState {
        match frame {
            ConsentFrame::Withdraw(_) => state.withdraw().unwrap_or(state),
            ConsentFrame::Suspend(_) => state.suspend().unwrap_or(state),
            ConsentFrame::Resume(_) => state.resume().unwrap_or(state),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
//  STATE MACHINE TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn state_granted_suspend() {
    assert_eq!(ConsentState::Granted.suspend(), Ok(ConsentState::Suspended));
}

#[test]
fn state_suspended_resume() {
    assert_eq!(ConsentState::Suspended.resume(), Ok(ConsentState::Granted));
}

#[test]
fn state_granted_withdraw() {
    assert_eq!(ConsentState::Granted.withdraw(), Ok(ConsentState::Withdrawn));
}

#[test]
fn state_suspended_withdraw() {
    assert_eq!(ConsentState::Suspended.withdraw(), Ok(ConsentState::Withdrawn));
}

#[test]
fn state_withdrawn_is_terminal() {
    assert_eq!(ConsentState::Withdrawn.suspend(), Err(TransitionError::AlreadyWithdrawn));
    assert_eq!(ConsentState::Withdrawn.resume(), Err(TransitionError::AlreadyWithdrawn));
    assert_eq!(ConsentState::Withdrawn.withdraw(), Err(TransitionError::AlreadyWithdrawn));
}

#[test]
fn state_idempotent_suspend() {
    // TV-009: double suspend is no-op
    assert_eq!(ConsentState::Suspended.suspend(), Ok(ConsentState::Suspended));
}

#[test]
fn state_idempotent_resume_from_granted() {
    // TV-010: resume from granted is no-op
    assert_eq!(ConsentState::Granted.resume(), Ok(ConsentState::Granted));
}

#[test]
fn state_cognitive_frames_allowed() {
    assert!(ConsentState::Granted.allows_cognitive_frames());
    assert!(!ConsentState::Suspended.allows_cognitive_frames());
    assert!(!ConsentState::Withdrawn.allows_cognitive_frames());
}

#[test]
fn state_gossip_bits_roundtrip() {
    for state in [ConsentState::Granted, ConsentState::Suspended, ConsentState::Withdrawn] {
        let bits = state.to_gossip_bits();
        let recovered = ConsentState::from_gossip_bits(bits).unwrap();
        assert_eq!(state, recovered, "Gossip bits round-trip failed for {:?}", state);
    }
    assert_eq!(ConsentState::from_gossip_bits(0b11), None); // reserved
}

// ═══════════════════════════════════════════════════════════════════
//  ENGINE TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn engine_register_and_query() {
    let mut engine = ConsentEngine::new();
    let peer = [1u8; 16];
    engine.register_peer(peer, 1000).unwrap();
    assert_eq!(engine.get_state(&peer), Some(ConsentState::Granted));
}

#[test]
fn engine_duplicate_registration_rejected() {
    let mut engine = ConsentEngine::new();
    let peer = [2u8; 16];
    engine.register_peer(peer, 1000).unwrap();
    assert!(engine.register_peer(peer, 2000).is_err());
}

#[test]
fn engine_peer_table_full() {
    let mut engine = ConsentEngine::new();
    for i in 0..MAX_PEERS {
        let mut peer = [0u8; 16];
        peer[0] = i as u8;
        engine.register_peer(peer, 1000).unwrap();
    }
    let extra = [0xFFu8; 16];
    assert!(engine.register_peer(extra, 1000).is_err());
}

#[test]
fn engine_suspend_resume_cycle() {
    let mut engine = ConsentEngine::new();
    let peer = [3u8; 16];
    engine.register_peer(peer, 1000).unwrap();

    // Suspend
    let s = engine.suspend(&peer, Some(ReasonCode::UserInitiated), 2000).unwrap();
    assert_eq!(s, ConsentState::Suspended);
    assert!(!engine.allows_cognitive_frames(&peer));

    // Resume
    let s = engine.resume(&peer, 3000).unwrap();
    assert_eq!(s, ConsentState::Granted);
    assert!(engine.allows_cognitive_frames(&peer));
}

#[test]
fn engine_withdraw_is_terminal() {
    let mut engine = ConsentEngine::new();
    let peer = [4u8; 16];
    engine.register_peer(peer, 1000).unwrap();

    let s = engine.withdraw(&peer, Some(ReasonCode::SafetyViolation), 2000).unwrap();
    assert_eq!(s, ConsentState::Withdrawn);
    assert!(!engine.allows_cognitive_frames(&peer));

    // Cannot suspend after withdrawal
    assert!(engine.suspend(&peer, None, 3000).is_err());
}

#[test]
fn engine_withdraw_all() {
    let mut engine = ConsentEngine::new();
    let p1 = [5u8; 16];
    let p2 = [6u8; 16];
    let p3 = [7u8; 16];
    engine.register_peer(p1, 1000).unwrap();
    engine.register_peer(p2, 1000).unwrap();
    engine.register_peer(p3, 1000).unwrap();

    let count = engine.withdraw_all(Some(ReasonCode::EmergencyButton), 2000);
    assert_eq!(count, 3);
    assert_eq!(engine.get_state(&p1), Some(ConsentState::Withdrawn));
    assert_eq!(engine.get_state(&p2), Some(ConsentState::Withdrawn));
    assert_eq!(engine.get_state(&p3), Some(ConsentState::Withdrawn));
}

#[test]
fn engine_unknown_peer_rejected() {
    let mut engine = ConsentEngine::new();
    let unknown = [0xFFu8; 16];
    assert_eq!(engine.suspend(&unknown, None, 1000), Err(TransitionError::PeerNotFound));
    assert_eq!(engine.resume(&unknown, 1000), Err(TransitionError::PeerNotFound));
    assert_eq!(engine.withdraw(&unknown, None, 1000), Err(TransitionError::PeerNotFound));
}

#[test]
fn engine_unknown_peer_cognitive_frames_rejected() {
    let engine = ConsentEngine::new();
    let unknown = [0xFFu8; 16];
    assert!(!engine.allows_cognitive_frames(&unknown));
}

// ═══════════════════════════════════════════════════════════════════
//  REASON CODE TESTS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn reason_code_spec_reserved_range() {
    assert!(ReasonCode::Unspecified.is_spec_reserved());
    assert!(ReasonCode::UserInitiated.is_spec_reserved());
    assert!(ReasonCode::SafetyViolation.is_spec_reserved());
    assert!(ReasonCode::HardwareFault.is_spec_reserved());
}

#[test]
fn reason_code_implementation_specific_range() {
    assert!(ReasonCode::StimGuardLockout.is_implementation_specific());
    assert!(ReasonCode::SessionAttestationFailure.is_implementation_specific());
    assert!(ReasonCode::EmergencyButton.is_implementation_specific());
    assert!(ReasonCode::SwarmFaultDetected.is_implementation_specific());
}

#[test]
fn reason_code_unknown_defaults_to_unspecified() {
    // TV-012: unknown reasonCode 0xFF treated as Unspecified
    assert_eq!(ReasonCode::from_u8(0xFF), ReasonCode::Unspecified);
    assert_eq!(ReasonCode::from_u8(0x42), ReasonCode::Unspecified);
}

#[test]
fn reason_code_u8_roundtrip() {
    for code in [
        ReasonCode::Unspecified, ReasonCode::UserInitiated,
        ReasonCode::SafetyViolation, ReasonCode::HardwareFault,
        ReasonCode::StimGuardLockout, ReasonCode::EmergencyButton,
    ] {
        assert_eq!(ReasonCode::from_u8(code.to_u8()), code);
    }
}
