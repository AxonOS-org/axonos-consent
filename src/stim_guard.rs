//! StimGuard integration for bidirectional BCI.
//!
//! When consent is withdrawn, StimGuard closes the hardware DAC gate
//! via Secure GPIO, physically disconnecting the stimulation output
//! from the electrode connector.
//!
//! Enforcement path (Section 8 of consent spec):
//! 1. Frame parser (NS) calls nsc_withdraw_consent()
//! 2. ConsentEngine.set_withdrawn() — state update in Secure World
//! 3. StimGuard.consent_withdrawn() — closes DAC gate
//! 4. Steps 2-3 are atomic in Secure World, <1µs combined
//!
//! This module is feature-gated behind `stim-guard`.

/// Hardware abstraction for the Secure GPIO DAC gate.
///
/// In production: maps to STM32H573 Secure GPIO register.
/// In testing: no-op or mock.
pub trait DacGate {
    /// Close the DAC gate — physically disconnect stimulation output.
    /// WCET: single register write, <0.1µs.
    fn close(&mut self);

    /// Open the DAC gate (only on explicit re-consent after power cycle).
    fn open(&mut self);

    /// Query gate state.
    fn is_closed(&self) -> bool;
}

/// StimGuard consent integration.
///
/// Called by ConsentEngine on withdrawal. This is the bridge between
/// the protocol layer (consent state machine) and the hardware layer
/// (physical DAC gate isolation).
pub struct StimGuardConsent<G: DacGate> {
    gate: G,
    lockout_active: bool,
}

impl<G: DacGate> StimGuardConsent<G> {
    pub fn new(gate: G) -> Self {
        Self {
            gate,
            lockout_active: false,
        }
    }

    /// Called by ConsentEngine when any peer withdraws consent.
    /// Closes the DAC gate immediately. No conditions, no delays.
    ///
    /// This is the consent-withdraw → StimGuard lockout path
    /// documented in the AxonOS conformance profile.
    pub fn on_consent_withdrawn(&mut self) {
        self.gate.close();
        self.lockout_active = true;
    }

    /// Query whether stimulation lockout is active.
    pub fn is_locked_out(&self) -> bool {
        self.lockout_active
    }

    /// Re-enable stimulation (only after power cycle + re-consent).
    /// NOT callable during normal operation — requires explicit re-handshake.
    pub fn clear_lockout(&mut self) {
        self.lockout_active = false;
        self.gate.open();
    }
}
