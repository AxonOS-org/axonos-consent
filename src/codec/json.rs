//! JSON codec for consent frames.
//!
//! Used at the relay boundary — MMP reference implementations (sym, sym-swift)
//! use length-prefixed JSON over TCP/WebSocket.
//!
//! Also used by the #[test] harness to load interop test vectors.
//! Supports full round-trip: decode JSON → ConsentFrame → encode JSON.

use alloc::string::String;
use crate::frames::*;
use crate::reason::ReasonCode;

/// Decode a ConsentFrame from a serde_json::Value (parsed JSON object).
///
/// This is the entry point for loading test vectors:
/// ```ignore
/// let v: serde_json::Value = serde_json::from_str(json_str)?;
/// let frame = json::decode_value(&v)?;
/// ```
#[cfg(feature = "json")]
pub fn decode_value(v: &serde_json::Value) -> Result<ConsentFrame, String> {
    let obj = v.as_object().ok_or_else(|| String::from("expected JSON object"))?;

    let frame_type = obj.get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| String::from("missing 'type' field"))?;

    let reason_code = obj.get("reasonCode")
        .and_then(|v| v.as_u64())
        .map(|v| ReasonCode::from_u8(v as u8));

    let reason = obj.get("reason")
        .and_then(|v| v.as_str())
        .map(String::from);

    let timestamp_ms = obj.get("timestamp")
        .and_then(|v| v.as_u64());

    let timestamp_us = obj.get("timestamp_us")
        .and_then(|v| v.as_u64());

    match frame_type {
        "consent-withdraw" => {
            let scope_str = obj.get("scope")
                .and_then(|v| v.as_str())
                .ok_or_else(|| String::from("missing 'scope' field"))?;
            let scope = Scope::from_str(scope_str)
                .ok_or_else(|| alloc::format!("unknown scope: {}", scope_str))?;

            let epoch = obj.get("epoch")
                .and_then(|v| v.as_u64());

            Ok(ConsentFrame::Withdraw(ConsentWithdraw {
                scope, reason_code, reason, epoch, timestamp_ms, timestamp_us,
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
        other => Err(alloc::format!("unknown frame type: {}", other)),
    }
}

/// Encode a ConsentFrame to a serde_json::Value (JSON object).
#[cfg(feature = "json")]
pub fn encode_value(frame: &ConsentFrame) -> serde_json::Value {
    let mut map = serde_json::Map::new();

    match frame {
        ConsentFrame::Withdraw(w) => {
            map.insert("type".into(), "consent-withdraw".into());
            map.insert("scope".into(), w.scope.as_str().into());
            if let Some(rc) = w.reason_code {
                map.insert("reasonCode".into(), (rc.to_u8() as u64).into());
            }
            if let Some(ref r) = w.reason {
                map.insert("reason".into(), r.clone().into());
            }
            if let Some(e) = w.epoch {
                map.insert("epoch".into(), e.into());
            }
            if let Some(ts) = w.timestamp_ms {
                map.insert("timestamp".into(), ts.into());
            }
            if let Some(ts) = w.timestamp_us {
                map.insert("timestamp_us".into(), ts.into());
            }
        }
        ConsentFrame::Suspend(s) => {
            map.insert("type".into(), "consent-suspend".into());
            if let Some(rc) = s.reason_code {
                map.insert("reasonCode".into(), (rc.to_u8() as u64).into());
            }
            if let Some(ref r) = s.reason {
                map.insert("reason".into(), r.clone().into());
            }
            if let Some(ts) = s.timestamp_ms {
                map.insert("timestamp".into(), ts.into());
            }
            if let Some(ts) = s.timestamp_us {
                map.insert("timestamp_us".into(), ts.into());
            }
        }
        ConsentFrame::Resume(r) => {
            map.insert("type".into(), "consent-resume".into());
            if let Some(ts) = r.timestamp_ms {
                map.insert("timestamp".into(), ts.into());
            }
            if let Some(ts) = r.timestamp_us {
                map.insert("timestamp_us".into(), ts.into());
            }
        }
    }

    serde_json::Value::Object(map)
}

/// Encode a ConsentFrame to canonical JSON string.
#[cfg(feature = "json")]
pub fn encode(frame: &ConsentFrame) -> Result<String, serde_json::Error> {
    serde_json::to_string(&encode_value(frame))
}

/// Decode a ConsentFrame from a JSON string.
#[cfg(feature = "json")]
pub fn decode(json_str: &str) -> Result<ConsentFrame, String> {
    let v: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| alloc::format!("JSON parse error: {}", e))?;
    decode_value(&v)
}
