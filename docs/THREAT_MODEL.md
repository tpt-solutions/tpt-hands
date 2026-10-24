<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 TPT Solutions -->

# Threat model: power handshake

**Status: draft, not reviewed.** This model covers the reference handshake in
`crates/tpt-hands-handshake`. It is not audited. The Phase 3 security review and
the formal proofs (Kani, TLA+) are still to come, so do not rely on this model to
protect real power until they are done.

## Scope

In scope:

- The handshake between a device (end-effector) and its power source, as in
  DESIGN.md section 7: hello, challenge, response, relay close.
- Key use for signing the response.
- The replay counter the source keeps per device.

Out of scope for now:

- Key provisioning, storage, rotation and revocation. The design is still open
  (todo.md, Phase 3).
- The on-device relay driver and its timing (< 1 ms link-loss open).
- Transport security for the telemetry link. Covered by tpt-link.
- Physical attacks such as glitching the MCU or probing the key.

## Assets

1. **Relay state.** Closed means power is on. A wrong close is the main hazard.
2. **Device keys.** Ed25519 signing keys enrolled per `tool_id`.
3. **Replay counters.** Per-device monotonic counter kept by the source.

## Trust boundaries

- The power source (host side) decides whether to close the relay.
- The device proves its identity by signing. It is not trusted just because it
  is plugged in.
- The cable carries untrusted bytes in both directions.

## Threats and current mitigations

| # | Threat | Mitigation in the reference crate | Gap |
| - | ------ | --------------------------------- | --- |
| T1 | Attacker replays an old, valid response | Counter must strictly increase per device; refused handshakes do not advance it | Counter storage across power loss is not designed |
| T2 | Attacker signs with a key they do not own | Signature verified against the key enrolled for `tool_id` | Enrolment process is not designed |
| T3 | Signature reused in another protocol | Domain tag `tpt-hands/handshake/v1` and both nonces are signed | Tag must be bumped on any format change |
| T4 | Stalled or slow handshake holds the relay in an unknown state | Timeout rejects a handshake that runs too long; any failure leaves the relay open | Timeout value not yet set for hardware |
| T5 | Predictable challenge nonce | Caller supplies the nonce; the crate has no RNG | Caller must use a CSPRNG; not enforced by the type |
| T6 | Cable pulled mid-handshake | Failure leaves the relay open, which is the de-energised safe state | Relay open time on hardware not measured |
| T7 | Corrupted frames | Frame CRC in `tpt-hands-proto::frame` | CRC is not a MAC; it detects noise, not tampering |
| T8 | Clock skew | Handshake uses counters and timeouts, not wall-clock time | Timeout source on the MCU is not specified |

## Assumptions

- Ed25519 (`ed25519-dalek`) is correct and its verification is strict enough
  for this use. Not independently checked.
- The source's counter store survives reset and is not writable by the device.
- The device's private key never leaves the device.

## What the failure default is

Any error, timeout, bad signature or replayed counter leaves the relay **open**.
Power is only ever enabled by a fully verified handshake.

## Open items

- [ ] Key provisioning, storage, rotation and revocation design
- [ ] Persistent counter storage design
- [ ] Formal model (TLA+) of handshake and link loss
- [ ] Kani proofs: relay closed implies data link alive; no close without a valid signature
- [ ] Independent security review
- [ ] Physical fault-injection test (hardware)
