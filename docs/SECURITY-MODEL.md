# Security model (informative)

This document expands on the threat model in SPEC §11. It is **informative**;
SPEC §11 is normative.

---

## 1. What is being defended

The consent subsystem defends the user's ability to **revoke or pause** the
flow of `IntentObservation` from kernel to application. The defended property
is: "after the user signals withdrawal through the trusted path, no further
observation reaches the application within 10 ms wall-clock time."

## 2. The trust anchors

The subsystem trusts:

- The **kernel** to honour the state machine and the publication gate.
- The **trusted path** to deliver only events that actually came from the user.
- The **secure element** to protect the trusted-path public key from extraction.

Compromise of any one of these defeats the consent guarantee. The Cognitive
Hypervisor (TrustZone-M Secure World) is the layer responsible for protecting
the first two against software-level compromise; the secure element (ATECC608B)
is the hardware layer for the third.

## 3. What the subsystem does **not** defend against

### 3.1 An attacker with kernel-image replacement

If the attacker can flash a modified kernel image, the consent state machine
is whatever the modified kernel says it is. Defence is Secure Boot: the
boot ROM verifies the kernel image signature before jumping to it.

### 3.2 An attacker with physical replacement of the trusted path

If the attacker swaps the hardware button for a remote-controlled relay, the
consent system sees a button press that did not come from the user. Defence
is device-level tamper detection: a tamper-evident enclosure with a switch
that triggers a panic on opening.

### 3.3 Side-channel inference of cognitive state

A malicious application with a legitimate `Navigation` capability could
infer cognitive state from response-time patterns to displayed stimuli.
This is an application-layer attack on the user, not a consent-system
failure; defending against it requires application-layer review and is out
of scope for the consent subsystem.

### 3.4 A legitimate but malicious application

If a user installs an application that declares legitimate capabilities and
then exfiltrates the data it is admitted to receive, the consent system has
done its job — the application received only what its manifest declared,
and the user installed it. What the application does with received
observations is the application's responsibility.

## 4. The honest-but-buggy application

The consent system explicitly defends against this case. A bug in the
application's UI code (or a failure of the application to handle a
`ConsentSuspended` error gracefully) cannot cause data to flow when consent
is suspended, because the gate is at the kernel publication path, not the
application code.

This is the central reason for placing consent in the kernel.

## 5. The Foundation-level safety case

For a clinical deployment, the consent subsystem's role in the safety case is:

> *Claim:* the user can withdraw consent at any time, and within ≤ 10 ms
> wall-clock time no further observation reaches the application.
> *Evidence:* SPEC §6 wire format, SPEC §3.1 transition graph, L1 Kani
> proofs of timing bounds, L2 soak-test traces on reference hardware.
> *Counter-evidence:* a Kani counterexample (none exists at v0.3.0); a
> measurement on reference hardware exceeding 10 ms (none exists at v0.3.0).

The Cognitive Hypervisor's interlock (SPEC §9.1.3) extends this guarantee
to stimulation deployments.
