<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 TPT Solutions -->

# Changelog

All notable changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/) as described in
[docs/RELEASING.md](docs/RELEASING.md). Nothing is published yet, so everything is
under **Unreleased**.

## Unreleased

### Added

- **SDK.** `EndEffector` trait, capability and telemetry types, the
  `SimulatedHand` mock with healing, growth and hypertrophy, and a 112-line
  example of a new end-effector.
- **Protobuf.** Provisional schema in `proto/`, with Rust (micropb) and Go
  bindings generated from it. A compatibility check runs against a baseline
  snapshot.
- **Frames and transports.** Frame format with CRC and sequence numbers, from
  tpt-link. Host-side serial, UDP and CAN (fragmentation) support.
- **Edge reflexes.** Host-testable `no_std` logic: debounced pathway monitor,
  fail-soft route pick, grip overload and slip reflex.
- **Power handshake.** Reference state machine with Ed25519 signatures, replay
  counters checked at hello and at response, and a fault-injection simulator.
  Not audited. See `docs/HANDSHAKE.md` and `docs/THREAT_MODEL.md`.
- **Morphogenesis simulation.** In-house single-joint model with tear, healing,
  reinforcement and grasp, plus a scripted baseline agent.
- **Chrysalis adapter.** Request mapping, outcome reading and a job lifecycle
  state machine. HTTP client for the proposed device-command interface, tested
  against a labelled mock. The real tpt-chrysalis service does not have this
  endpoint yet.
- **Rust gRPC.** `tpt-hands-grpc`: unary calls over cleartext HTTP/2 with hyper
  and micropb. Deadlines, status trailers, message limits.
- **Go gRPC.** `go/tpt-grpc-lite`: unary and streaming over HTTP/2, TLS and
  mTLS, deadlines, metadata, reconnect. Interoperates with grpc-core and with
  `tpt-hands-grpc`.
- **Go orchestrator.** `/healthz` and `/version`, JSON logging, environment
  config, graceful shutdown.
- **Integration.** Healing loop and failure-mode scenarios, and a growth loop, all
  against mocks. `examples/sim_healing_loop` prints the loop as a timeline.
- **Tests and checks.** Fuzz targets for frame decoding and forged handshake
  responses, with smoke tests on stable. A 20,000-message link soak test with
  injected faults. Benchmarks for the reflex tick and the UDP link.
- **Licence policy.** `cargo deny` for the Rust workspace, `go-licenses` for the
  Go modules, and `THIRD_PARTY_LICENSES.md` from `cargo about`.

### Changed

- The Rust host uses its own gRPC layer on hyper and micropb. tonic is not used,
  because it depends on `sync_wrapper`, which is Apache-2.0 only.

### Fixed

- The handshake verifier now checks the replay counter when a response arrives,
  not only when a hello arrives. Two overlapping handshakes could otherwise both
  close the relay with one counter. See THREAT_MODEL T9.
- `tpt-hands-proto`'s `alloc` feature now enables `tpt-link/alloc`, so frame
  encoding is available to any crate that uses the `alloc` feature.

### Known limits

- No hardware has run any of this. Timing numbers are host numbers.
- The protobuf schema is provisional, and it is not yet reconciled with the real
  tpt-link schema.
- The tpt-soma adapter is a placeholder. There is no soma ingest path yet.
- The power handshake has no formal proofs, and it is not audited.
