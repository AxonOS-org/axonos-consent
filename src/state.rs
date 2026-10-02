// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! The consent state machine: three states, seven admissible transitions.
//!
//! This module is pure. It holds no state and performs no I/O; the machines in
//! [`crate::machine`] and [`crate::dual_control`] own the state and apply these
//! rules to it.

use crate::error::ConsentError;

/// One of the three consent states (SPEC §2.1).
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum ConsentState {
    /// Observations flow.
    Granted = 0x01,
    /// Observations do not flow. Resumable through the trusted path.
    Suspended = 0x02,
    /// Observations never flow again for this installation. Terminal.
    Withdrawn = 0x03,
}

impl ConsentState {
    /// Decode a discriminant received on the wire. Strict: any other byte is
    /// refused (SPEC §2.1).
    pub const fn from_wire(byte: u8) -> Result<Self, ConsentError> {
        match byte {
            0x01 => Ok(Self::Granted),
            0x02 => Ok(Self::Suspended),
            0x03 => Ok(Self::Withdrawn),
            _ => Err(ConsentError::ReservedDiscriminant),
        }
    }

    /// Decode a byte read back from storage or from the publication gate.
    /// Fail-closed: a byte that is not `Granted` or `Suspended` reads as
    /// `Withdrawn`, so corruption can stop observations but never start them
    /// (SPEC §2.3).
    pub const fn from_stored(byte: u8) -> Self {
        match byte {
            0x01 => Self::Granted,
            0x02 => Self::Suspended,
            _ => Self::Withdrawn,
        }
    }

    /// The discriminant byte.
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// True for `Withdrawn`, the only terminal state.
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Withdrawn)
    }
}

/// Is `from → to` admissible (SPEC §3.1)?
///
/// Seven transitions are: the three identities, pause and resume, and
/// withdrawal from either active state. The two transitions out of `Withdrawn`
/// are not.
pub const fn is_admissible_transition(from: ConsentState, to: ConsentState) -> bool {
    use ConsentState::{Granted, Suspended, Withdrawn};
    matches!(
        (from, to),
        (Granted, Granted)
            | (Suspended, Suspended)
            | (Withdrawn, Withdrawn)
            | (Granted, Suspended)
            | (Suspended, Granted)
            | (Granted, Withdrawn)
            | (Suspended, Withdrawn)
    )
}

/// Does `from → to` increase exposure (SPEC §12.3)? Only `Suspended → Granted`
/// does; under dual control it needs both parties.
pub const fn is_exposure_increasing(from: ConsentState, to: ConsentState) -> bool {
    matches!((from, to), (ConsentState::Suspended, ConsentState::Granted))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ConsentState::{Granted, Suspended, Withdrawn};

    const ALL: [ConsentState; 3] = [Granted, Suspended, Withdrawn];

    #[test]
    fn exactly_seven_transitions_are_admissible() {
        let admitted = ALL
            .iter()
            .flat_map(|&f| ALL.iter().map(move |&t| (f, t)))
            .filter(|&(f, t)| is_admissible_transition(f, t))
            .count();
        assert_eq!(admitted, 7);
        assert!(!is_admissible_transition(Withdrawn, Granted));
        assert!(!is_admissible_transition(Withdrawn, Suspended));
    }

    #[test]
    fn only_resume_increases_exposure() {
        for f in ALL {
            for t in ALL {
                assert_eq!(is_exposure_increasing(f, t), (f, t) == (Suspended, Granted));
            }
        }
    }

    #[test]
    fn wire_decoding_is_strict() {
        for s in ALL {
            assert_eq!(ConsentState::from_wire(s.as_u8()), Ok(s));
        }
        for b in [0x00u8, 0x04, 0x7F, 0xFF] {
            assert_eq!(
                ConsentState::from_wire(b),
                Err(ConsentError::ReservedDiscriminant)
            );
        }
    }

    #[test]
    fn stored_decoding_fails_closed() {
        for b in 0..=u8::MAX {
            let s = ConsentState::from_stored(b);
            match b {
                0x01 => assert_eq!(s, Granted),
                0x02 => assert_eq!(s, Suspended),
                _ => assert_eq!(s, Withdrawn),
            }
        }
    }
}
