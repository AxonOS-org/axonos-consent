// SPDX-License-Identifier: Apache-2.0 OR MIT
// SPDX-FileCopyrightText: 2026 Denis Yermakou <connect@axonos.org>

//! The publication gate: consent and publication in one atomic word (SPEC §9).
//!
//! Reading the consent state and then publishing an observation is a race: a
//! withdrawal can land between the read and the write, and an observation is
//! then published after consent was withdrawn. The gate closes that window by
//! keeping the consent state and the count of published observations in the
//! same `AtomicU32`:
//!
//! ```text
//!  bits 31..2   published   observations committed so far (mod 2^30)
//!  bits  1..0   state       01 Granted · 10 Suspended · 11 Withdrawn
//! ```
//!
//! A publication commits with one compare-and-swap that succeeds only while
//! the state bits read `Granted`. A withdrawal rewrites the state bits of the
//! same word. Both are operations on one atomic object, so they are totally
//! ordered: once a withdrawal has stored `Withdrawn` in the word, no
//! [`PublicationGate::try_publish`] can succeed, and no observation committed
//! after that point can ever become visible. That ordering is the linearization
//! point SPEC §9.2 requires, and the `loom` tests below check it under every
//! interleaving the C11 memory model allows.
//!
//! The producer protocol for one SPSC ring:
//!
//! 1. write the observation into slot `published() % capacity`;
//! 2. call `try_publish()`. `Ok` makes the slot visible; `Err` means consent
//!    is not granted, and the slot is abandoned unread.
//!
//! The consumer reads `published()` with acquire ordering and reads slots up to
//! it. Ring capacity must be a power of two no larger than 2^29, so that the
//! 30-bit count wraps cleanly.

#[cfg(not(loom))]
use core::sync::atomic::{AtomicU32, Ordering};
#[cfg(loom)]
use loom::sync::atomic::{AtomicU32, Ordering};

use crate::state::ConsentState;

const STATE_MASK: u32 = 0b11;
const COUNT_SHIFT: u32 = 2;
/// The publication count is kept modulo `COUNT_MASK + 1` (2^30).
pub const COUNT_MASK: u32 = (1 << 30) - 1;

/// Why a publication was refused.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum Suppressed {
    /// Consent is suspended.
    Suspended,
    /// Consent is withdrawn, or the gate word was corrupted (fail-closed).
    Withdrawn,
}

impl Suppressed {
    /// The error the SDK delivers (AxonOS Standard, Section 20): `0x05` suspended,
    /// `0x06` withdrawn.
    pub const fn abi_code(self) -> u8 {
        match self {
            Self::Suspended => 0x05,
            Self::Withdrawn => 0x06,
        }
    }
}

/// Consent state and publication count in one atomic word. See the module
/// documentation for the protocol.
#[derive(Debug)]
pub struct PublicationGate {
    word: AtomicU32,
}

impl PublicationGate {
    /// A gate in the `Granted` state with nothing published.
    #[cfg(not(loom))]
    pub const fn new() -> Self {
        Self::with_state(ConsentState::Granted)
    }

    /// A gate in the `Granted` state with nothing published.
    #[cfg(loom)]
    pub fn new() -> Self {
        Self::with_state(ConsentState::Granted)
    }

    #[cfg(not(loom))]
    pub(crate) const fn with_state(state: ConsentState) -> Self {
        Self {
            word: AtomicU32::new(state as u32),
        }
    }

    #[cfg(loom)]
    pub(crate) fn with_state(state: ConsentState) -> Self {
        Self {
            word: AtomicU32::new(state as u32),
        }
    }

    /// The consent state, read fail-closed: corrupted state bits read as
    /// `Withdrawn`.
    pub fn state(&self) -> ConsentState {
        state_of(self.word.load(Ordering::Acquire))
    }

    /// Observations committed so far, modulo 2^30.
    pub fn published(&self) -> u32 {
        self.word.load(Ordering::Acquire) >> COUNT_SHIFT
    }

    /// Commit one observation if, and only if, consent is granted at the
    /// instant of the commit. Returns the index of the committed slot.
    pub fn try_publish(&self) -> Result<u32, Suppressed> {
        let mut current = self.word.load(Ordering::Acquire);
        loop {
            match state_of(current) {
                ConsentState::Granted => {}
                ConsentState::Suspended => return Err(Suppressed::Suspended),
                ConsentState::Withdrawn => return Err(Suppressed::Withdrawn),
            }
            let index = current >> COUNT_SHIFT;
            let next = (((index + 1) & COUNT_MASK) << COUNT_SHIFT) | (current & STATE_MASK);
            match self
                .word
                .compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return Ok(index),
                Err(actual) => current = actual,
            }
        }
    }

    /// Store a new consent state, keeping the count. `Withdrawn` is absorbing:
    /// once stored, no later call can leave it. Crate-internal; only the
    /// machines, after authentication, change the state.
    pub(crate) fn set_state(&self, state: ConsentState) {
        let mut current = self.word.load(Ordering::Acquire);
        loop {
            // Withdrawn is absorbing, and corrupted state bits read as
            // Withdrawn, so both stay (or become) Withdrawn.
            let target = if state_of(current).is_terminal() {
                ConsentState::Withdrawn
            } else {
                state
            };
            let next = (current & !STATE_MASK) | target as u32;
            if next == current {
                return;
            }
            match self
                .word
                .compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return,
                Err(actual) => current = actual,
            }
        }
    }

    #[cfg(kani)]
    pub(crate) fn from_raw(raw: u32) -> Self {
        Self {
            word: AtomicU32::new(raw),
        }
    }

    #[cfg(kani)]
    pub(crate) fn raw(&self) -> u32 {
        self.word.load(Ordering::Acquire)
    }
}

#[cfg(not(loom))]
impl Default for PublicationGate {
    fn default() -> Self {
        Self::new()
    }
}

const fn state_of(word: u32) -> ConsentState {
    ConsentState::from_stored((word & STATE_MASK) as u8)
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;

    #[test]
    fn publishes_only_while_granted() {
        let g = PublicationGate::new();
        assert_eq!(g.try_publish(), Ok(0));
        assert_eq!(g.try_publish(), Ok(1));
        g.set_state(ConsentState::Suspended);
        assert_eq!(g.try_publish(), Err(Suppressed::Suspended));
        assert_eq!(g.published(), 2);
        g.set_state(ConsentState::Granted);
        assert_eq!(g.try_publish(), Ok(2));
    }

    #[test]
    fn withdrawal_is_absorbing() {
        let g = PublicationGate::new();
        g.set_state(ConsentState::Withdrawn);
        g.set_state(ConsentState::Granted);
        g.set_state(ConsentState::Suspended);
        assert_eq!(g.state(), ConsentState::Withdrawn);
        assert_eq!(g.try_publish(), Err(Suppressed::Withdrawn));
    }

    #[test]
    fn count_wraps_without_touching_state() {
        let g = PublicationGate {
            word: AtomicU32::new((COUNT_MASK << COUNT_SHIFT) | ConsentState::Granted as u32),
        };
        assert_eq!(g.try_publish(), Ok(COUNT_MASK));
        assert_eq!(g.published(), 0);
        assert_eq!(g.state(), ConsentState::Granted);
    }

    #[test]
    fn corrupted_state_bits_fail_closed() {
        let g = PublicationGate {
            word: AtomicU32::new(5 << COUNT_SHIFT),
        };
        assert_eq!(g.state(), ConsentState::Withdrawn);
        assert_eq!(g.try_publish(), Err(Suppressed::Withdrawn));
        g.set_state(ConsentState::Granted);
        assert_eq!(g.try_publish(), Err(Suppressed::Withdrawn));
    }
}

/// Model-checked under every interleaving the C11 memory model allows:
/// `RUSTFLAGS="--cfg loom" cargo test --release --lib loom`.
#[cfg(all(test, loom))]
mod loom_tests {
    use super::*;
    use loom::sync::atomic::AtomicU32 as Slot;
    use loom::sync::Arc;
    use loom::thread;

    /// Nothing becomes visible after a withdrawal has returned.
    #[test]
    fn loom_no_publication_after_withdrawal() {
        loom::model(|| {
            let gate = Arc::new(PublicationGate::new());
            let producer = {
                let gate = gate.clone();
                thread::spawn(move || {
                    let _ = gate.try_publish();
                    let _ = gate.try_publish();
                })
            };
            gate.set_state(ConsentState::Withdrawn);
            let at_withdrawal = gate.published();
            producer.join().unwrap();
            assert_eq!(gate.published(), at_withdrawal);
            assert_eq!(gate.state(), ConsentState::Withdrawn);
        });
    }

    /// A consumer that sees a publication also sees the data written before it.
    #[test]
    fn loom_publication_carries_the_slot() {
        loom::model(|| {
            let gate = Arc::new(PublicationGate::new());
            let slot = Arc::new(Slot::new(0));
            let producer = {
                let (gate, slot) = (gate.clone(), slot.clone());
                thread::spawn(move || {
                    slot.store(0xA5, Ordering::Relaxed);
                    let _ = gate.try_publish();
                })
            };
            if gate.published() == 1 {
                assert_eq!(slot.load(Ordering::Relaxed), 0xA5);
            }
            producer.join().unwrap();
        });
    }

    /// A suspension racing a publication either lets it through before, or
    /// refuses it after; it never loses the state.
    #[test]
    fn loom_suspension_races_publication() {
        loom::model(|| {
            let gate = Arc::new(PublicationGate::new());
            let producer = {
                let gate = gate.clone();
                thread::spawn(move || gate.try_publish())
            };
            gate.set_state(ConsentState::Suspended);
            let result = producer.join().unwrap();
            assert_eq!(gate.state(), ConsentState::Suspended);
            match result {
                Ok(0) => assert_eq!(gate.published(), 1),
                Err(Suppressed::Suspended) => assert_eq!(gate.published(), 0),
                other => panic!("unexpected {other:?}"),
            }
        });
    }
}
