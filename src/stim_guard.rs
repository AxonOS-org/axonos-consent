//! StimGuard integration for bidirectional BCI. Feature: `stim-guard`.

pub trait DacGate {
    fn close(&mut self);
    fn open(&mut self);
    fn is_closed(&self) -> bool;
}

pub struct StimGuardConsent<G: DacGate> {
    gate: G,
    lockout_active: bool,
}

impl<G: DacGate> StimGuardConsent<G> {
    pub fn new(gate: G) -> Self { Self { gate, lockout_active: false } }

    pub fn on_consent_withdrawn(&mut self) {
        self.gate.close();
        self.lockout_active = true;
    }

    pub fn is_locked_out(&self) -> bool { self.lockout_active }

    pub fn clear_lockout(&mut self) {
        self.lockout_active = false;
        self.gate.open();
    }
}
