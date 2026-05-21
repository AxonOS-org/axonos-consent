//! L2 benchmark: withdrawal latency on the host CPU.
//!
//! Reference hardware bound is ≤ 1648 cycles (≈ 9.8 µs at 168 MHz Cortex-M4F).
//! Host CPUs are much faster, so this bench measures a lower bound; useful for
//! catching regressions.

use axonos_consent::crypto::compute_tag;
use axonos_consent::wire::FLAG_TERMINAL;
use axonos_consent::{ConsentEvent, ConsentMachine};
use std::time::Instant;

fn main() {
    const N: usize = 1_000_000;
    let pk = [0xAAu8; 32];

    let event = {
        let mut e = ConsentEvent {
            state: 0x03,
            flags: FLAG_TERMINAL,
            manifest_id: 1,
            timestamp_us: 1_000_000,
            sig_truncated: 0,
        };
        e.sig_truncated = compute_tag(&e, &pk);
        e
    };

    let start = Instant::now();
    for _ in 0..N {
        let mut m = ConsentMachine::new(1, pk);
        let _ = m.handle_event(event);
    }
    let elapsed = start.elapsed();

    println!("benchmark: {} withdrawals", N);
    println!("  total elapsed: {:?}", elapsed);
    println!("  per withdrawal: {:?}", elapsed / N as u32);
    println!("  bound on reference hardware: ≤ 9.8 µs (L1)");
}
