//! CBOR codec for consent frames.
//!
//! Compact binary encoding for local IPC (M4F ↔ A53).
//! No string parsing on the critical path — type field is encoded as
//! a string key per MMP spec, but CBOR encodes strings efficiently.
//!
//! Wire format: CBOR map with string keys matching JSON field names.
//! This ensures that CBOR-to-JSON transcoding at the relay boundary
//! is a mechanical transformation with no semantic interpretation.

use alloc::vec::Vec;
use crate::frames::*;
use crate::reason::ReasonCode;

/// Encode a ConsentFrame to CBOR bytes.
pub fn encode(frame: &ConsentFrame) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);

    match frame {
        ConsentFrame::Withdraw(w) => {
            // CBOR map: {"type": "consent-withdraw", "scope": "...", ...}
            let mut map_len: u64 = 2; // type + scope
            if w.reason_code.is_some() { map_len += 1; }
            if w.reason.is_some() { map_len += 1; }
            if w.epoch.is_some() { map_len += 1; }
            if w.timestamp_ms.is_some() { map_len += 1; }
            if w.timestamp_us.is_some() { map_len += 1; }

            encode_map_header(&mut buf, map_len);
            encode_text(&mut buf, "type");
            encode_text(&mut buf, "consent-withdraw");
            encode_text(&mut buf, "scope");
            encode_text(&mut buf, w.scope.as_str());

            if let Some(rc) = w.reason_code {
                encode_text(&mut buf, "reasonCode");
                encode_uint(&mut buf, rc.to_u8() as u64);
            }
            if let Some(ref r) = w.reason {
                encode_text(&mut buf, "reason");
                encode_text(&mut buf, r);
            }
            if let Some(e) = w.epoch {
                encode_text(&mut buf, "epoch");
                encode_uint(&mut buf, e);
            }
            if let Some(ts) = w.timestamp_ms {
                encode_text(&mut buf, "timestamp");
                encode_uint(&mut buf, ts);
            }
            if let Some(ts) = w.timestamp_us {
                encode_text(&mut buf, "timestamp_us");
                encode_uint(&mut buf, ts);
            }
        }

        ConsentFrame::Suspend(s) => {
            let mut map_len: u64 = 1; // type
            if s.reason_code.is_some() { map_len += 1; }
            if s.reason.is_some() { map_len += 1; }
            if s.timestamp_ms.is_some() { map_len += 1; }
            if s.timestamp_us.is_some() { map_len += 1; }

            encode_map_header(&mut buf, map_len);
            encode_text(&mut buf, "type");
            encode_text(&mut buf, "consent-suspend");

            if let Some(rc) = s.reason_code {
                encode_text(&mut buf, "reasonCode");
                encode_uint(&mut buf, rc.to_u8() as u64);
            }
            if let Some(ref r) = s.reason {
                encode_text(&mut buf, "reason");
                encode_text(&mut buf, r);
            }
            if let Some(ts) = s.timestamp_ms {
                encode_text(&mut buf, "timestamp");
                encode_uint(&mut buf, ts);
            }
            if let Some(ts) = s.timestamp_us {
                encode_text(&mut buf, "timestamp_us");
                encode_uint(&mut buf, ts);
            }
        }

        ConsentFrame::Resume(r) => {
            let mut map_len: u64 = 1; // type
            if r.timestamp_ms.is_some() { map_len += 1; }
            if r.timestamp_us.is_some() { map_len += 1; }

            encode_map_header(&mut buf, map_len);
            encode_text(&mut buf, "type");
            encode_text(&mut buf, "consent-resume");

            if let Some(ts) = r.timestamp_ms {
                encode_text(&mut buf, "timestamp");
                encode_uint(&mut buf, ts);
            }
            if let Some(ts) = r.timestamp_us {
                encode_text(&mut buf, "timestamp_us");
                encode_uint(&mut buf, ts);
            }
        }
    }

    buf
}

// === Minimal CBOR encoder (no external dependency for core encoding) ===

fn encode_map_header(buf: &mut Vec<u8>, len: u64) {
    encode_type_value(buf, 5, len); // major type 5 = map
}

fn encode_text(buf: &mut Vec<u8>, s: &str) {
    encode_type_value(buf, 3, s.len() as u64); // major type 3 = text string
    buf.extend_from_slice(s.as_bytes());
}

fn encode_uint(buf: &mut Vec<u8>, v: u64) {
    encode_type_value(buf, 0, v); // major type 0 = unsigned integer
}

fn encode_type_value(buf: &mut Vec<u8>, major: u8, value: u64) {
    let mt = major << 5;
    if value < 24 {
        buf.push(mt | value as u8);
    } else if value <= 0xFF {
        buf.push(mt | 24);
        buf.push(value as u8);
    } else if value <= 0xFFFF {
        buf.push(mt | 25);
        buf.extend_from_slice(&(value as u16).to_be_bytes());
    } else if value <= 0xFFFF_FFFF {
        buf.push(mt | 26);
        buf.extend_from_slice(&(value as u32).to_be_bytes());
    } else {
        buf.push(mt | 27);
        buf.extend_from_slice(&value.to_be_bytes());
    }
}
