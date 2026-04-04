//! CBOR codec for consent frames.
//!
//! Compact binary encoding for local IPC (M4F ↔ A53).
//! Wire format: CBOR map with string keys matching JSON field names.
//! This ensures CBOR-to-JSON transcoding at the relay boundary
//! is a mechanical transformation with no semantic interpretation.
//!
//! Supports full round-trip: encode → wire → decode → assert equality.

use alloc::string::String;
use alloc::vec::Vec;
use crate::frames::*;
use crate::reason::ReasonCode;

// ═══════════════════════════════════════════════════════════════════
//  ENCODER
// ═══════════════════════════════════════════════════════════════════

/// Encode a ConsentFrame to CBOR bytes.
pub fn encode(frame: &ConsentFrame) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);

    match frame {
        ConsentFrame::Withdraw(w) => {
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
            let mut map_len: u64 = 1;
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
            let mut map_len: u64 = 1;
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

// ═══════════════════════════════════════════════════════════════════
//  DECODER
// ═══════════════════════════════════════════════════════════════════

/// Decode error types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    UnexpectedEof,
    InvalidCbor,
    ExpectedMap,
    ExpectedTextKey,
    MissingTypeField,
    UnknownFrameType(String),
    MissingScopeField,
    UnknownScope(String),
}

/// Decode a ConsentFrame from CBOR bytes. Round-trip counterpart to `encode`.
pub fn decode(data: &[u8]) -> Result<ConsentFrame, DecodeError> {
    let mut cur = Cursor::new(data);
    let map_len = cur.read_map_len()?;

    let mut frame_type: Option<String> = None;
    let mut scope: Option<String> = None;
    let mut reason_code: Option<ReasonCode> = None;
    let mut reason: Option<String> = None;
    let mut epoch: Option<u64> = None;
    let mut timestamp_ms: Option<u64> = None;
    let mut timestamp_us: Option<u64> = None;

    for _ in 0..map_len {
        let key = cur.read_text()?;
        match key.as_str() {
            "type"        => { frame_type = Some(cur.read_text()?); }
            "scope"       => { scope = Some(cur.read_text()?); }
            "reasonCode"  => { reason_code = Some(ReasonCode::from_u8(cur.read_uint()? as u8)); }
            "reason"      => { reason = Some(cur.read_text()?); }
            "epoch"       => { epoch = Some(cur.read_uint()?); }
            "timestamp"   => { timestamp_ms = Some(cur.read_uint()?); }
            "timestamp_us"=> { timestamp_us = Some(cur.read_uint()?); }
            _             => { cur.skip_value()?; } // forward-compat: ignore unknown keys
        }
    }

    let ft = frame_type.ok_or(DecodeError::MissingTypeField)?;

    match ft.as_str() {
        "consent-withdraw" => {
            let scope_str = scope.ok_or(DecodeError::MissingScopeField)?;
            let s = Scope::from_str(&scope_str)
                .ok_or_else(|| DecodeError::UnknownScope(scope_str))?;
            Ok(ConsentFrame::Withdraw(ConsentWithdraw {
                scope: s, reason_code, reason, epoch, timestamp_ms, timestamp_us,
            }))
        }
        "consent-suspend" => {
            Ok(ConsentFrame::Suspend(ConsentSuspend {
                reason_code, reason, timestamp_ms, timestamp_us,
            }))
        }
        "consent-resume" => {
            Ok(ConsentFrame::Resume(ConsentResume {
                timestamp_ms, timestamp_us,
            }))
        }
        other => Err(DecodeError::UnknownFrameType(String::from(other))),
    }
}

// ═══════════════════════════════════════════════════════════════════
//  CBOR PRIMITIVES
// ═══════════════════════════════════════════════════════════════════

fn encode_map_header(buf: &mut Vec<u8>, len: u64) { encode_type_value(buf, 5, len); }
fn encode_text(buf: &mut Vec<u8>, s: &str) {
    encode_type_value(buf, 3, s.len() as u64);
    buf.extend_from_slice(s.as_bytes());
}
fn encode_uint(buf: &mut Vec<u8>, v: u64) { encode_type_value(buf, 0, v); }

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

struct Cursor<'a> { data: &'a [u8], pos: usize }

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self { Self { data, pos: 0 } }

    fn read_byte(&mut self) -> Result<u8, DecodeError> {
        self.data.get(self.pos).copied().map(|b| { self.pos += 1; b })
            .ok_or(DecodeError::UnexpectedEof)
    }

    fn advance(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.pos + n > self.data.len() { return Err(DecodeError::UnexpectedEof); }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn read_argument(&mut self, ai: u8) -> Result<u64, DecodeError> {
        match ai {
            0..=23 => Ok(ai as u64),
            24 => Ok(self.read_byte()? as u64),
            25 => { let b = self.advance(2)?; Ok(u16::from_be_bytes([b[0], b[1]]) as u64) }
            26 => { let b = self.advance(4)?; Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as u64) }
            27 => { let b = self.advance(8)?; Ok(u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])) }
            _ => Err(DecodeError::InvalidCbor),
        }
    }

    fn read_uint(&mut self) -> Result<u64, DecodeError> {
        let ib = self.read_byte()?;
        if ib >> 5 != 0 { return Err(DecodeError::InvalidCbor); }
        self.read_argument(ib & 0x1F)
    }

    fn read_text(&mut self) -> Result<String, DecodeError> {
        let ib = self.read_byte()?;
        if ib >> 5 != 3 { return Err(DecodeError::ExpectedTextKey); }
        let len = self.read_argument(ib & 0x1F)? as usize;
        let bytes = self.advance(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| DecodeError::InvalidCbor)
    }

    fn read_map_len(&mut self) -> Result<u64, DecodeError> {
        let ib = self.read_byte()?;
        if ib >> 5 != 5 { return Err(DecodeError::ExpectedMap); }
        self.read_argument(ib & 0x1F)
    }

    fn skip_value(&mut self) -> Result<(), DecodeError> {
        let ib = self.read_byte()?;
        let major = ib >> 5;
        let arg = self.read_argument(ib & 0x1F)?;
        match major {
            0 | 1 => {} // int — already consumed
            2 | 3 => { self.advance(arg as usize)?; } // bytes/text
            4 => { for _ in 0..arg { self.skip_value()?; } } // array
            5 => { for _ in 0..arg { self.skip_value()?; self.skip_value()?; } } // map
            6 => { self.skip_value()?; } // tag
            7 => {} // simple/float
            _ => return Err(DecodeError::InvalidCbor),
        }
        Ok(())
    }
}
