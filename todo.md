# tpt-hands — Project TODO

Licence: **MIT OR Apache-2.0** (dual) · Copyright holder: **TPT Solutions**
Dependency rule: permissive only; **no Apache-2.0-only dependencies** (MIT, BSD, ISC, Zlib, and `MIT OR Apache-2.0` dual are OK).

Decisions locked in:
- Coupling to tpt-soma / tpt-chrysalis: gRPC/Protobuf only, no crate deps on those repos
- Sim: minimal in-house soft-body model (no MuJoCo/Genesis)
- Software only, no hardware CAD/reference design
- MCUs: ESP32 + STM32 · Transports: Serial/USB-CDC, UDP, CAN
- Ecosystem repos (soma, chrysalis, cerebrum, swarm, link) already exist with stable APIs
- Go orchestrator included in v1.0 (thin service)

Licence findings (checked 2026-10-16, re-verify with cargo-deny / go-licenses):
- Apache-2.0 only, BANNED: `prost`, `prost-build`, `tonic-prost` (MIT itself but depends on prost), `grpc-go`, connect-go
- OK: `tonic` (MIT, supports custom codecs), `h2` (MIT), `micropb` + `micropb-gen` (MIT OR Apache-2.0, no_std), `protobuf` 4.x (BSD-3), `google.golang.org/protobuf` (BSD-3), `golang.org/x/net/http2` (BSD-3)

Protobuf/gRPC stack decision:
- Rust host: `tonic` with a custom `Codec` + `micropb` (or `protobuf` 4.x) message types; no prost anywhere in the tree
- Firmware (`no_std`): `micropb`
- Go: `google.golang.org/protobuf` (BSD-3) + custom minimal gRPC client/server over `golang.org/x/net/http2` (write ourselves; no grpc-go)

STM32 reference family: **STM32G4** (e.g. G474/G431) — embassy-stm32 support, FDCAN, USB. ESP32 reference: **ESP32-C6** (RISC-V, stable Rust).

---

## Phase 0 — Foundation, Licensing & Traits (Weeks 1–2)

### Repo & legal
- [x] Init git repo, `.gitignore`, `.gitattributes`
- [x] Add `LICENSE-MIT` and `LICENSE-APACHE` (copyright: TPT Solutions)
- [x] Add per-file SPDX licence headers / REUSE config (`REUSE.toml` plus `LICENSES/`; `reuse lint` passes)
- [x] Set `license = "MIT OR Apache-2.0"` in all Cargo.toml files and `go` module metadata (Cargo via workspace inheritance; Go modules have no licence field, so the SPDX header is the equivalent)
- [x] Add README licence section + CONTRIBUTING (contributions dual-licensed by default)
- [ ] Add `NOTICE`/`THIRD_PARTY_LICENSES` generation (cargo-about / go-licenses) (cargo-about installed; needs deps to generate from)
- [x] Add CODE_OF_CONDUCT, SECURITY.md, issue/PR templates (the CoC and SECURITY.md contact address still need confirming)
- [x] Move `spec.txt` to `DESIGN.md` (and fix it to match decisions above)

### Dependency licence policy
- [x] Add `deny.toml` (cargo-deny): allow MIT, BSD-2/3, ISC, Zlib, Unicode, `MIT OR Apache-2.0`; deny Apache-2.0-only and copyleft
- [ ] Add Go licence check (go-licenses) with same allow list
- [ ] Re-verify licences of the full transitive tree for tonic + custom codec + micropb (cargo-deny) and protoc/protobuf-go toolchain
- [ ] Spike: tonic client/server with custom `Codec` backed by micropb; confirm no prost in `cargo tree`
- [ ] Spike: micropb-gen vs `protobuf` 4.x for host-side types; pick one and document
- [x] Write `tpt-grpc-lite` Go package: gRPC framing (5-byte length-prefixed messages), HTTP/2 via x/net/http2, trailers/status codes, unary + server/client/bidi streaming, deadlines, metadata (`go/tpt-grpc-lite`; tests and benchmarks pass, see its README)
- [x] Go: TLS/mTLS, keepalive, and reconnect support in `tpt-grpc-lite` (TLS and mTLS tested; reconnect tested across a server restart; keepalive tested against a frozen TCP peer, which fails the test when removed)
- [ ] Interop tests: Go `tpt-grpc-lite` ↔ Rust tonic, and both ↔ the real tpt-soma / tpt-chrysalis / tpt-swarm servers (wire-compat with their grpc). Both directions pass against grpc-core (Python `grpc`), an independent implementation. Rust tonic and the real tpt-soma / tpt-chrysalis / tpt-swarm servers are still pending.
- [x] Decide whether `tpt-grpc-lite` is a separate repo/module (reusable across TPT repos): separate Go module in this repo, at `go/tpt-grpc-lite`
- [ ] Verify that the existing TPT repos' APIs can be consumed with the chosen stack; if any of them require features we skip (e.g. grpc-web, compression), list them
- [ ] Wire licence checks into CI as a required gate (licence job added to `ci.yml`; still needs branch protection to make it required)

### Workspace & tooling
- [x] Create Cargo workspace root with all crates from the spec layout
- [x] Pin Rust toolchain (`rust-toolchain.toml`), MSRV policy (pinned 1.97.0; `rust-version = 1.85` in workspace; written policy pending)
- [x] rustfmt, clippy (deny warnings), `cargo doc` config
- [x] Go module setup under `go/tpt-hands-orchestrator` (module path `github.com/tpt-solutions/tpt-hands/...`, matching origin)
- [x] CI (GitHub Actions): build, test, fmt, clippy, deny, Go vet/test, on Linux + Windows (licence and REUSE jobs included)
- [x] Pre-commit hooks / justfile / xtask for common commands (`justfile` and `.pre-commit-config.yaml`)
- [ ] Import the real tpt-link, tpt-soma, tpt-chrysalis, tpt-cerebrum, tpt-swarm `.proto` files (vendored or submodule) and record versions (tpt-link: recreated locally, schema still to import; the others are public under TPT-Solutions)

### Core types & traits (`tpt-hands-sdk`)
- [x] Define `EndEffector` trait (`identity`, `capabilities`, `command`, `next_telemetry`, `physiological_state`)
- [x] Define `ToolCapabilities`, `ToolIntent`, `ToolTelemetry`, `ToolIdentity`, `ControlError`
- [x] Define `PhysiologicalState`, `PathwayHealth`, `PathwayState`, `HealingProtocolId`, `PathwayId`
- [x] Define `MorphogenesisRequest`, `MorphAction`, `SafetyEnvelope`, `FluidType`, `ChannelId`, `TendonId`
- [x] Decide telemetry channel type: pull model, `next_telemetry() -> Option<ToolTelemetry>`, bounded queue (no runtime dependency)
- [x] Decide sync vs async API: synchronous, so no async runtime is pulled in
- [ ] Decide `no_std` split (core types crate usable by firmware); SDK still uses `std`
- [ ] Unit tests and doc examples for every public type (done for the state machine, `ToolIdentity`, `ToolCapabilities`, the clocks and `SimulatedHand`; other types still need examples)
- [x] `SimulatedHand` mock driver
  - [x] Healing state machine (Intact → Severed → healing_progress → Intact)
  - [x] Growth state machine (PrepareForReinforcement → complete_reinforcement → 500 ms recalibration → updated capabilities)
  - [x] Hypertrophied pathway state
  - [x] Deterministic clock injection for tests (`ManualClock`)
- [x] Write a "new end-effector in < 200 lines" example and measure it (`crates/tpt-hands-sdk/examples/new_end_effector.rs`: 112 lines)
- [ ] Publish `tpt-hands-sdk` v0.1 (crates.io dry-run, then publish)

---

## Phase 1 — Edge Reflexes & Protocol (Weeks 3–5)

### Wire protocol (`tpt-hands-proto`)
- [ ] Reconcile spec Protobuf messages with the real tpt-link schema (PROVISIONAL draft at `proto/tpt_hands.proto`, transcribed from DESIGN.md section 6; no real tpt-link `.proto` is available on this machine. Gap found: the request has no id to match the ack, so `request_id` was added as field 2.)
- [x] Define `PhysiologicalUpdate`, `PathwayHealth`, `MorphogenesisRequest`, `MorphogenesisAck`, `SafetyEnvelope` and action messages (provisional, see the reconcile item above)
- [x] Add telemetry, command/intent and power-handshake messages (PROVISIONAL, drafted from the spec wording. Handshake shapes only: the security design is Phase 3. Gap: `command_id` added to `ToolCommand` so commands can be acked. 8 golden fixtures from Go protobuf.)
- [ ] Define versioning/compat rules and a `buf`-style breaking-change check
- [x] Generated code: Rust via micropb-gen 0.5.1 (MIT OR Apache-2.0), no prost. `tools/proto-gen` regenerates `crates/tpt-hands-proto/src/generated/` (protoc 29.3). CI fails if the committed code is stale. Verified against 9 golden fixtures from Go protobuf.
- [x] Generated code for Go (protoc-gen-go, BSD-3) in `go/tpt-hands-orchestrator/internal/gen/tpthandsv1` (`just proto-go`; round-trip tests pass. Generated with protoc 35.1; the Rust side uses 29.3.)
- [ ] Round-trip and fuzz tests for encode/decode (round-trip and exhaustive bit-flip tests done; fuzzing pending)
- [x] Frame format: length prefix, CRC, sequence numbers, version byte (`tpt-hands-proto::frame`, CRC-32 in-house)

### Transports (`tpt-link`)
- [x] Transport abstraction trait (`tpt-link::Transport`, datagram semantics; serial needs a stream reassembler)
- [ ] Serial / USB-CDC transport (host + firmware) (host side done: `tpt-link::serial::SerialTransport` behind the `serial` feature, with `Deframer` for byte-stream framing (tested without hardware); a real port round trip and firmware side are pending a bench)
- [x] UDP transport, host side (`tpt-link::UdpTransport`, std only, tested over localhost; firmware side pending)
- [ ] CAN transport (host via SocketCAN/PCAN adapter + firmware)
  - [x] Fragmentation/reassembly for messages larger than a CAN frame (`tpt-link::fragment`: split and reassemble, MTU is a parameter, up to 1280 bytes at classic MTU 8)
  - [x] CAN-FD support decision: classic CAN is the baseline. The fragment codec takes the MTU, so FD (64) is a configuration change, not a rewrite.
- [x] Heartbeat, link-alive detection, reconnect logic (`Heartbeat`, plus `reconnect::Reconnector` with exponential backoff: first try immediate, then 100 ms doubling to a 5 s cap in the recommended defaults. Wiring into the orchestrator is pending.)
- [ ] Loopback test harness for each transport (loopback and fault injection done; per-transport harnesses pending)

### Edge firmware (`tpt-hands-edge`)
- [ ] HAL abstraction layer so logic is shared across MCUs
- [ ] ESP32 target (RISC-V variant preferred) — esp-hal / embassy bring-up (`tpt-hands-edge` compiles for `riscv32imc-unknown-none-elf`; the C6 core is `riscv32imac`, so that target still needs installing; bring-up needs hardware)
- [ ] STM32 target — embassy-stm32 bring-up on STM32G4 (reference family) (`tpt-hands-edge` compiles for `thumbv7em-none-eabihf`; embassy bring-up needs hardware)
- [ ] 1 kHz reflex loop with timing/jitter measurement
- [x] Slip / overload reflexes (`reflex::GripReflex`, pure logic, tested on host; wired to force sensing on hardware pending)
- [ ] Pathway resistance monitoring (ADC drivers, filtering, thresholds) (debounced thresholds done in `reflex::PathwayMonitor`; ADC drivers and filtering pending)
- [x] Fail-soft load redistribution and telemetry rerouting (Pathway A → B) (`reflex::pick_route`)
- [x] Pathway state machine on-device (Intact / Severed / Hypertrophied) (debounced tear and healing; `PathwayMonitor::reinforce` marks a usable pathway Hypertrophied on host command, and it is still watched for tears)
- [ ] Local micro-calibration cycle (500 ms) after morphology change
- [ ] Watchdog, panic handler, safe-state defaults
- [ ] Flash/debug tooling docs (probe-rs, espflash)
- [ ] Hardware-in-the-loop CI plan (or documented manual test procedure)
- [ ] End-to-end loopback: simulated tear → edge detects → reported via telemetry

---

## Phase 2 — Ecosystem Adapters (Weeks 6–9)

### Soma adapter (`tpt-hands-soma-adapter`)
- [x] Read real tpt-soma API/proto; document mapping from `PhysiologicalState` (done: `docs/ECOSYSTEM_INTEGRATION.md`. Finding: tpt-soma is a patient-data platform. Its Flight `do_put` and `do_action` are unimplemented, so there is no ingest path for device telemetry.)
- [ ] Implement gRPC client streaming `PhysiologicalUpdate`
- [ ] Reconnect, backpressure and buffering behaviour
- [ ] Mock tpt-soma server for tests
- [ ] Receive repair directives from soma where the API supports it

### Chrysalis adapter (`tpt-hands-chrysalis-adapter`)
- [x] Read real tpt-chrysalis API/proto; document mapping from `MorphogenesisRequest` (done: `docs/ECOSYSTEM_INTEGRATION.md`. Finding: chrysalis has no heal, conduit or tendon actions. Its vocabulary is Dispense, SetValve and SetTemperature, and its API is a Phase 1 dev surface. The mapping is by analogy only.)
- [ ] Implement gRPC client for submitting requests and receiving `MorphogenesisAck`
- [ ] Track `chrysalis_job_id` lifecycle (submitted, approved, running, done, failed)
- [ ] Safety envelope serialization and rejection handling
- [ ] Mock tpt-chrysalis server for tests

### Go orchestrator (`go/tpt-hands-orchestrator`)
- [ ] Read tpt-swarm API; define bridge contract
- [ ] Thin service: swarm ↔ SDK, device registry, routing by `tool_id`
- [ ] Health/metrics endpoint, structured logging, config (done: `/healthz` and `/version`, JSON `slog`, env config with startup validation, graceful shutdown. Metrics endpoint pending.)
- [ ] Cerebrum-facing intent path (capability queries, prepare-for-reinforcement)
- [ ] Integration tests against mocks
- [ ] Dockerfile / run instructions

### Integration
- [ ] Integration test: Tear → Soma Update → Chrysalis Request → Ack
- [ ] Integration test: Growth loop (cerebrum intent → ReinforceTendon → recalibration → updated capabilities)
- [ ] `examples/sim_healing_loop` end-to-end mock
- [ ] `docs/ECOSYSTEM_INTEGRATION.md`
- [ ] Failure-mode tests (soma down, chrysalis rejects, link drop mid-heal)

---

## Phase 3 — Smart Power & Formal Proofs (Weeks 10–13)

### Handshake implementation
- [ ] Design handshake protocol spec (data pins mate first → challenge/response → relay close)
- [x] Choose Ed25519 library that passes licence policy and runs `no_std` (ed25519-dalek 3.0.0, BSD-3-Clause; the `no_std` build is not checked yet)
- [ ] Key provisioning, storage, rotation and revocation design
- [x] Replay protection (`tpt-hands-handshake`: counter strictly increases per device; a refused handshake does not advance it)
- [ ] Implement handshake state machine on tpt-edge (ESP32 + STM32)
- [ ] Relay driver with hardware-enforced default-open
- [ ] Link-loss detection → relay open in < 1 ms
- [ ] Host-side handshake implementation in SDK (the state machine is in `crates/tpt-hands-handshake`; wiring into the SDK and to the relay driver is pending)
- [ ] Threat model document (draft in `docs/THREAT_MODEL.md`; not yet reviewed)

### Formal verification
- [ ] Kani proofs: `relay_closed ⇒ data_link_alive`
- [ ] Kani proofs: no relay close without a valid signature
- [ ] TLA+ model of handshake + link-loss, checked with TLC
- [ ] Run proofs in CI
- [ ] Document assumptions and what is not covered by the proofs

### Tooling & testing
- [ ] CLI simulator for handshake fault injection
- [ ] Fault cases: bad signature, replay, mid-handshake cable pull, clock skew, corrupted frames
- [ ] Physical fault-injection test procedure (cable yanked → relay opens in < 1 ms) and recorded result
- [ ] `examples/smart_socket_bringup`
- [ ] Security review of the handshake and key handling

---

## Phase 4 — Sim2Real Morphogenesis (Weeks 14–16)

### In-house simulation model (`sim/morphogenesis_plugin`)
- [x] Define scope of the soft-body/stiffness/mass model (what fidelity is "enough"): one joint, penalty contact, semi-implicit Euler. Stated in `sim/morphogenesis_plugin/src/lib.rs`.
- [x] Choose implementation language and licence-clean dependencies (math/linear algebra): Rust, no physics dependencies, only the SDK path dependency.
- [x] Hand/finger model with joints, stiffness, mass (single finger joint done; the full hand is not modelled yet)
- [x] Apply `MorphogenesisAck` effects: tendon thickening raises stiffness (`reinforce`), healing restores integrity. Mass change on heal is not modelled.
- [x] Healing timer driven by simulated time
- [x] Injury injection API (tear a pathway): `Finger::injure`
- [x] Simple grasp task environment (`run_grasp`). Lift is not modelled yet; the grasp holds a contact force for a set time.
- [x] Integrate with `SimulatedHand` and the SDK (`SimHand` implements `EndEffector`). Over protobuf: not done.
- [x] Validation against analytic/simple test cases (free equilibrium, grasp equilibrium, reinforcement, release)
- [ ] Optional visualisation (licence-checked)

### Demo
- [x] Agent "injures" a finger, waits for healing timer, completes the task (scripted test in the model; a demo recording is not made)
- [x] Scripted baseline agent (no dependency on tpt-cerebrum): `morphogenesis_plugin::baseline::run_scripted`, with `examples/baseline_agent.rs`. The cerebrum-driven run is not done.
- [ ] Record demo and write walkthrough

---

## Phase 5 — Release Hardening to v1.0

### Docs
- [x] `docs/ADDING_A_TOOL.md`
- [ ] API docs complete and published
- [ ] Architecture overview, wire protocol reference, power handshake spec
- [ ] Firmware porting guide (adding another MCU)
- [ ] Wiring/bring-up guide for ESP32 and STM32 dev boards (no CAD)

### Quality
- [ ] Test coverage review; add missing tests
- [ ] Fuzz targets for frame parsing and handshake messages
- [ ] Soak test: long-running link with fault injection
- [x] Performance benchmarks: reflex loop latency, link throughput (host-only, on loopback: `cargo bench -p tpt-hands-edge --bench reflex_tick`, `cargo bench -p tpt-hands-proto --bench link_throughput`, plus `go test -bench` in `go/tpt-grpc-lite`. MCU and real-link numbers still need hardware.)
- [ ] Final licence audit (all crates + Go modules + vendored protos) and `THIRD_PARTY_LICENSES`
- [ ] Semver and release process, CHANGELOG
- [ ] Publish all crates; tag v1.0

### v1.0 success criteria checklist
- [ ] New end-effector implementable in < 200 lines of Rust
- [ ] Simulated hand: tear → soma notified → chrysalis repair → recovery, no human intervention
- [ ] Power handshake formally verified, and fault-injection test passes (relay opens in < 1 ms)
- [ ] `tpt-hands-chrysalis-adapter` works against the real tpt-chrysalis repo API

---

- [x] **tpt-link is real and in use.** It lives at github.com/tpt-solutions/tpt-link (also cloned at C:\Programming	pt-link). tpt-hands depends on it at commit 8e23498 (v0.2.0 release; the tag is not pushed yet). Schema reconciled against proto/tpt_link.proto (package tpt.link.v1): no clashes. Open: request_id and command_id are candidates for the core (tpt-link spec section 13), a decision for the core maintainer.

- [ ] Confirm protobuf/gRPC stack passes the Apache-only rule (blocking for Phase 1; see Phase 0 spikes)
- [ ] Decide CAN vs CAN-FD (STM32G4 has FDCAN; ESP32-C6 is classic CAN only, so likely classic CAN as baseline, FD optional)
- [ ] Decide whether `no_std` core types live in their own crate
- [x] Serial transport crate, decided. `serialport` is MPL-2.0 (checked with `cargo info`), so it is banned. Adopted `serial2` 0.2.38 (BSD-2-Clause OR Apache-2.0), behind the optional `serial` feature of `tpt-link`. Rejected: `tokio-serial` and `mio-serial` (MIT, but pull in an async runtime), `serial-rs` (GPL-2.0-or-later). `cargo deny` passes with all features.
- [x] `tpt-hands-proto::frame` `no_std` decode path: decoding needs only `core`, and encoding is behind the `alloc` feature. `tpt-hands-edge` uses it and compiles for both MCU targets.
