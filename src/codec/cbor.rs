//! CBOR codec with security bounds for untrusted input.
//!
//! ## Security properties
//!
//! - `MAX_MAP_FIELDS = 8`: rejects maps larger than consent frame spec allows
//! - `MAX_STRING_LEN = 128`: bounds string allocation (reason field)
//! - `MAX_NESTING_DEPTH = 2`: prevents recursive structure attacks
//! - Duplicate key detection: rejects frames with repeated keys
//! - Forward-compatible: unknown keys are skipped (bounded)
//!
//! Zero-alloc: uses `ReasonBuf` (64-byte fixed buffer) not `String`.

use crate::frames::*;
use crate::reason::ReasonCode;

// ═══════════════════════════════════════════════════════════════════
//  SECURITY LIMITS
// ═══════════════════════════════════════════════════════════════════

/// Maximum number of key-value pairs in a consent frame map.
/// Consent frames have at most 7 fields (type, scope, reasonCode, reason, epoch, timestamp, timestamp_us).
pub const MAX_MAP_FIELDS: u64 = 8;

/// Maximum byte length for any text string (keys + values).
pub const MAX_STRING_LEN: usize = 128;

/// Maximum nesting depth for skip_value (prevents stack overflow from crafted input).
const MAX_NESTING_DEPTH: u8 = 4;

// ═══════════════════════════════════════════════════════════════════
//  ERRORS
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    UnexpectedEof,
    InvalidCbor,
    ExpectedMap,
    ExpectedText,
    MissingTypeField,
    UnknownFrameType,
    MissingScopeField,
    UnknownScope,
    MapTooLarge,
    StringTooLong,
    NestingTooDeep,
    DuplicateKey,
}

// ═══════════════════════════════════════════════════════════════════
//  ENCODER (zero-alloc, writes to caller-provided buffer)
// ═══════════════════════════════════════════════════════════════════

/// Maximum encoded size of any consent frame. Conservative upper bound.
pub const MAX_ENCODED_SIZE: usize = 256;

/// Encode a ConsentFrame into a fixed buffer. Returns bytes written.
pub fn encode(frame: &ConsentFrame, out: &mut [u8; MAX_ENCODED_SIZE]) -> usize {
    let mut w = Writer { buf: out, pos: 0 };

    match frame {
        ConsentFrame::Withdraw(f) => {
            let n = 2 + f.reason_code.is_some() as u64 + f.reason.is_some() as u64
                + f.epoch.is_some() as u64 + f.timestamp_ms.is_some() as u64
                + f.timestamp_us.is_some() as u64;
            w.map(n); w.text("type"); w.text("consent-withdraw");
            w.text("scope"); w.text(f.scope.as_str());
            if let Some(rc) = f.reason_code { w.text("reasonCode"); w.uint(rc.to_u8() as u64); }
            if let Some(ref r) = f.reason { w.text("reason"); w.text(r.as_str()); }
            if let Some(e) = f.epoch { w.text("epoch"); w.uint(e); }
            if let Some(t) = f.timestamp_ms { w.text("timestamp"); w.uint(t); }
            if let Some(t) = f.timestamp_us { w.text("timestamp_us"); w.uint(t); }
        }
        ConsentFrame::Suspend(f) => {
            let n = 1 + f.reason_code.is_some() as u64 + f.reason.is_some() as u64
                + f.timestamp_ms.is_some() as u64 + f.timestamp_us.is_some() as u64;
            w.map(n); w.text("type"); w.text("consent-suspend");
            if let Some(rc) = f.reason_code { w.text("reasonCode"); w.uint(rc.to_u8() as u64); }
            if let Some(ref r) = f.reason { w.text("reason"); w.text(r.as_str()); }
            if let Some(t) = f.timestamp_ms { w.text("timestamp"); w.uint(t); }
            if let Some(t) = f.timestamp_us { w.text("timestamp_us"); w.uint(t); }
        }
        ConsentFrame::Resume(f) => {
            let n = 1 + f.timestamp_ms.is_some() as u64 + f.timestamp_us.is_some() as u64;
            w.map(n); w.text("type"); w.text("consent-resume");
            if let Some(t) = f.timestamp_ms { w.text("timestamp"); w.uint(t); }
            if let Some(t) = f.timestamp_us { w.text("timestamp_us"); w.uint(t); }
        }
    }
    w.pos
}

// ═══════════════════════════════════════════════════════════════════
//  DECODER (bounded, duplicate-key-safe)
// ═══════════════════════════════════════════════════════════════════

/// Decode a ConsentFrame from CBOR bytes. Security-bounded.
pub fn decode(data: &[u8]) -> Result<ConsentFrame, DecodeError> {
    let mut c = Cursor { data, pos: 0 };
    let map_len = c.read_map_len()?;
    if map_len > MAX_MAP_FIELDS { return Err(DecodeError::MapTooLarge); }

    // Duplicate key tracking: bit flags for known keys
    // bit 0=type, 1=scope, 2=reasonCode, 3=reason, 4=epoch, 5=timestamp, 6=timestamp_us
    let mut seen: u8 = 0;

    let mut frame_type: Option<FrameType> = None;
    let mut scope: Option<Scope> = None;
    let mut reason_code: Option<ReasonCode> = None;
    let mut reason: Option<ReasonBuf> = None;
    let mut epoch: Option<u64> = None;
    let mut timestamp_ms: Option<u64> = None;
    let mut timestamp_us: Option<u64> = None;

    for _ in 0..map_len {
        let key = c.read_text_bounded()?;
        match key {
            "type" => {
                if seen & 1 != 0 { return Err(DecodeError::DuplicateKey); }
                seen |= 1;
                frame_type = Some(match c.read_text_bounded()? {
                    "consent-withdraw" => FrameType::Withdraw,
                    "consent-suspend" => FrameType::Suspend,
                    "consent-resume" => FrameType::Resume,
                    _ => return Err(DecodeError::UnknownFrameType),
                });
            }
            "scope" => {
                if seen & 2 != 0 { return Err(DecodeError::DuplicateKey); }
                seen |= 2;
                scope = Some(Scope::from_str(c.read_text_bounded()?)
                    .ok_or(DecodeError::UnknownScope)?);
            }
            "reasonCode" => {
                if seen & 4 != 0 { return Err(DecodeError::DuplicateKey); }
                seen |= 4;
                reason_code = Some(ReasonCode::from_u8(c.read_uint()? as u8));
            }
            "reason" => {
                if seen & 8 != 0 { return Err(DecodeError::DuplicateKey); }
                seen |= 8;
                reason = Some(ReasonBuf::from_str(c.read_text_bounded()?));
            }
            "epoch" => {
                if seen & 16 != 0 { return Err(DecodeError::DuplicateKey); }
                seen |= 16;
                epoch = Some(c.read_uint()?);
            }
            "timestamp" => {
                if seen & 32 != 0 { return Err(DecodeError::DuplicateKey); }
                seen |= 32;
                timestamp_ms = Some(c.read_uint()?);
            }
            "timestamp_us" => {
                if seen & 64 != 0 { return Err(DecodeError::DuplicateKey); }
                seen |= 64;
                timestamp_us = Some(c.read_uint()?);
            }
            _ => { c.skip_value(0)?; } // forward-compat
        }
    }

    let ft = frame_type.ok_or(DecodeError::MissingTypeField)?;
    match ft {
        FrameType::Withdraw => {
            let s = scope.ok_or(DecodeError::MissingScopeField)?;
            Ok(ConsentFrame::Withdraw(ConsentWithdraw {
                scope: s, reason_code, reason, epoch, timestamp_ms, timestamp_us,
            }))
        }
        FrameType::Suspend => Ok(ConsentFrame::Suspend(ConsentSuspend {
            reason_code, reason, timestamp_ms, timestamp_us,
        })),
        FrameType::Resume => Ok(ConsentFrame::Resume(ConsentResume {
            timestamp_ms, timestamp_us,
        })),
    }
}

#[derive(Clone, Copy)]
enum FrameType { Withdraw, Suspend, Resume }

// ═══════════════════════════════════════════════════════════════════
//  CBOR PRIMITIVES
// ═══════════════════════════════════════════════════════════════════

struct Writer<'a> { buf: &'a mut [u8; MAX_ENCODED_SIZE], pos: usize }

impl<'a> Writer<'a> {
    fn put(&mut self, b: u8) { if self.pos < MAX_ENCODED_SIZE { self.buf[self.pos] = b; self.pos += 1; } }
    fn put_slice(&mut self, s: &[u8]) { for &b in s { self.put(b); } }

    fn type_val(&mut self, major: u8, v: u64) {
        let mt = major << 5;
        if v < 24 { self.put(mt | v as u8); }
        else if v <= 0xFF { self.put(mt | 24); self.put(v as u8); }
        else if v <= 0xFFFF { self.put(mt | 25); self.put_slice(&(v as u16).to_be_bytes()); }
        else if v <= 0xFFFF_FFFF { self.put(mt | 26); self.put_slice(&(v as u32).to_be_bytes()); }
        else { self.put(mt | 27); self.put_slice(&v.to_be_bytes()); }
    }

    fn map(&mut self, n: u64) { self.type_val(5, n); }
    fn text(&mut self, s: &str) { self.type_val(3, s.len() as u64); self.put_slice(s.as_bytes()); }
    fn uint(&mut self, v: u64) { self.type_val(0, v); }
}

struct Cursor<'a> { data: &'a [u8], pos: usize }

impl<'a> Cursor<'a> {
    fn byte(&mut self) -> Result<u8, DecodeError> {
        self.data.get(self.pos).copied().map(|b| { self.pos += 1; b }).ok_or(DecodeError::UnexpectedEof)
    }

    fn advance(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.pos + n > self.data.len() { return Err(DecodeError::UnexpectedEof); }
        let s = &self.data[self.pos..self.pos + n]; self.pos += n; Ok(s)
    }

    fn argument(&mut self, ai: u8) -> Result<u64, DecodeError> {
        match ai {
            0..=23 => Ok(ai as u64),
            24 => Ok(self.byte()? as u64),
            25 => { let b = self.advance(2)?; Ok(u16::from_be_bytes([b[0], b[1]]) as u64) }
            26 => { let b = self.advance(4)?; Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as u64) }
            27 => { let b = self.advance(8)?; Ok(u64::from_be_bytes([b[0],b[1],b[2],b[3],b[4],b[5],b[6],b[7]])) }
            _ => Err(DecodeError::InvalidCbor),
        }
    }

    fn read_uint(&mut self) -> Result<u64, DecodeError> {
        let ib = self.byte()?;
        if ib >> 5 != 0 { return Err(DecodeError::InvalidCbor); }
        self.argument(ib & 0x1F)
    }

    /// Read text string with MAX_STRING_LEN bound.
    fn read_text_bounded(&mut self) -> Result<&'a str, DecodeError> {
        let ib = self.byte()?;
        if ib >> 5 != 3 { return Err(DecodeError::ExpectedText); }
        let len = self.argument(ib & 0x1F)? as usize;
        if len > MAX_STRING_LEN { return Err(DecodeError::StringTooLong); }
        let bytes = self.advance(len)?;
        core::str::from_utf8(bytes).map_err(|_| DecodeError::InvalidCbor)
    }

    fn read_map_len(&mut self) -> Result<u64, DecodeError> {
        let ib = self.byte()?;
        if ib >> 5 != 5 { return Err(DecodeError::ExpectedMap); }
        self.argument(ib & 0x1F)
    }

    /// Skip one CBOR value with depth bound.
    fn skip_value(&mut self, depth: u8) -> Result<(), DecodeError> {
        if depth > MAX_NESTING_DEPTH { return Err(DecodeError::NestingTooDeep); }
        let ib = self.byte()?;
        let major = ib >> 5;
        let arg = self.argument(ib & 0x1F)?;
        match major {
            0 | 1 => {}
            2 | 3 => {
                if arg as usize > MAX_STRING_LEN { return Err(DecodeError::StringTooLong); }
                self.advance(arg as usize)?;
            }
            4 => { for _ in 0..arg.min(MAX_MAP_FIELDS) { self.skip_value(depth + 1)?; } }
            5 => { for _ in 0..arg.min(MAX_MAP_FIELDS) { self.skip_value(depth + 1)?; self.skip_value(depth + 1)?; } }
            6 => { self.skip_value(depth + 1)?; }
            7 => {}
            _ => return Err(DecodeError::InvalidCbor),
        }
        Ok(())
    }
}
