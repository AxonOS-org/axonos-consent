//! JSON codec for consent frames.
//!
//! Used at the relay boundary — MMP reference implementations (sym, sym-swift)
//! use length-prefixed JSON over TCP/WebSocket.
//!
//! This module produces the canonical JSON representation from Section 3
//! of the consent spec. The relay forwards these frames without interpretation.

#[cfg(feature = "json")]
use serde::Serialize;

use alloc::string::String;
use crate::frames::*;
use crate::reason::ReasonCode;

/// Serialize a ConsentFrame to canonical JSON string.
#[cfg(feature = "json")]
pub fn encode(frame: &ConsentFrame) -> Result<String, serde_json::Error> {
    match frame {
        ConsentFrame::Withdraw(w) => {
            let mut map = serde_json::Map::new();
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
            serde_json::to_string(&map)
        }
        ConsentFrame::Suspend(s) => {
            let mut map = serde_json::Map::new();
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
            serde_json::to_string(&map)
        }
        ConsentFrame::Resume(r) => {
            let mut map = serde_json::Map::new();
            map.insert("type".into(), "consent-resume".into());
            if let Some(ts) = r.timestamp_ms {
                map.insert("timestamp".into(), ts.into());
            }
            if let Some(ts) = r.timestamp_us {
                map.insert("timestamp_us".into(), ts.into());
            }
            serde_json::to_string(&map)
        }
    }
}
