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
- [ ] Write `tpt-grpc-lite` Go package: gRPC framing (5-byte length-prefixed messages), HTTP/2 via x/net/http2, trailers/status codes, unary + server/client/bidi streaming, deadlines, metadata
- [ ] Go: TLS/mTLS, keepalive, and reconnect support in `tpt-grpc-lite` (or explicitly scope out)
- [ ] Interop tests: Go `tpt-grpc-lite` ↔ Rust tonic, and both ↔ the real tpt-soma / tpt-chrysalis / tpt-swarm servers (wire-compat with their grpc)
- [ ] Decide whether `tpt-grpc-lite` is a separate repo/module (reusable across TPT repos)
- [ ] Verify that the existing TPT repos' APIs can be consumed with the chosen stack; if any of them require features we skip (e.g. grpc-web, compression), list them
- [ ] Wire licence checks into CI as a required gate (licence job added to `ci.yml`; still needs branch protection to make it required)

### Workspace & tooling
- [x] Create Cargo workspace root with all crates from the spec layout
- [x] Pin Rust toolchain (`rust-toolchain.toml`), MSRV policy (pinned 1.97.0; `rust-version = 1.85` in workspace; written policy pending)
- [x] rustfmt, clippy (deny warnings), `cargo doc` config
- [x] Go module setup under `go/tpt-hands-orchestrator` (module path `github.com/PhillipC05/tpt-hands/...` is a placeholder until the real repo location is confirmed)
- [x] CI (GitHub Actions): build, test, fmt, clippy, deny, Go vet/test, on Linux + Windows (licence and REUSE jobs included)
- [x] Pre-commit hooks / justfile / xtask for common commands (`justfile` and `.pre-commit-config.yaml`)
- [ ] Import the real tpt-link, tpt-soma, tpt-chrysalis, tpt-cerebrum, tpt-swarm `.proto` files (vendored or submodule) and record versions

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
- [ ] Write a "new end-effector in < 200 lines" example and measure it
- [ ] Publish `tpt-hands-sdk` v0.1 (crates.io dry-run, then publish)

---

## Phase 1 — Edge Reflexes & Protocol (Weeks 3–5)

### Wire protocol (`tpt-hands-proto`)
- [ ] Reconcile spec Protobuf messages with the real tpt-link schema
- [ ] Define `PhysiologicalUpdate`, `PathwayHealth`, `MorphogenesisRequest`, `MorphogenesisAck`, `SafetyEnvelope` and action messages
- [ ] Add telemetry, command/intent and power-handshake messages
- [ ] Define versioning/compat rules and a `buf`-style breaking-change check
- [ ] Generated code: Rust via micropb-gen (std and `no_std`), Go via protoc-gen-go (BSD-3); no prost
- [ ] Round-trip and fuzz tests for encode/decode
- [ ] Frame format: length prefix, CRC, sequence numbers, version byte

### Transports (`tpt-link`)
- [ ] Transport abstraction trait
- [ ] Serial / USB-CDC transport (host + firmware)
- [ ] UDP transport (host + sim + firmware where applicable)
- [ ] CAN transport (host via SocketCAN/PCAN adapter + firmware)
  - [ ] Fragmentation/reassembly for messages larger than a CAN frame
  - [ ] CAN-FD support decision
- [ ] Heartbeat, link-alive detection, reconnect logic
- [ ] Loopback test harness for each transport

### Edge firmware (`tpt-hands-edge`)
- [ ] HAL abstraction layer so logic is shared across MCUs
- [ ] ESP32 target (RISC-V variant preferred) — esp-hal / embassy bring-up
- [ ] STM32 target — embassy-stm32 bring-up on STM32G4 (reference family)
- [ ] 1 kHz reflex loop with timing/jitter measurement
- [ ] Slip / overload reflexes
- [ ] Pathway resistance monitoring (ADC drivers, filtering, thresholds)
- [ ] Fail-soft load redistribution and telemetry rerouting (Pathway A → B)
- [ ] Pathway state machine on-device (Intact / Severed / Hypertrophied)
- [ ] Local micro-calibration cycle (500 ms) after morphology change
- [ ] Watchdog, panic handler, safe-state defaults
- [ ] Flash/debug tooling docs (probe-rs, espflash)
- [ ] Hardware-in-the-loop CI plan (or documented manual test procedure)
- [ ] End-to-end loopback: simulated tear → edge detects → reported via telemetry

---

## Phase 2 — Ecosystem Adapters (Weeks 6–9)

### Soma adapter (`tpt-hands-soma-adapter`)
- [ ] Read real tpt-soma API/proto; document mapping from `PhysiologicalState`
- [ ] Implement gRPC client streaming `PhysiologicalUpdate`
- [ ] Reconnect, backpressure and buffering behaviour
- [ ] Mock tpt-soma server for tests
- [ ] Receive repair directives from soma where the API supports it

### Chrysalis adapter (`tpt-hands-chrysalis-adapter`)
- [ ] Read real tpt-chrysalis API/proto; document mapping from `MorphogenesisRequest`
- [ ] Implement gRPC client for submitting requests and receiving `MorphogenesisAck`
- [ ] Track `chrysalis_job_id` lifecycle (submitted, approved, running, done, failed)
- [ ] Safety envelope serialization and rejection handling
- [ ] Mock tpt-chrysalis server for tests

### Go orchestrator (`go/tpt-hands-orchestrator`)
- [ ] Read tpt-swarm API; define bridge contract
- [ ] Thin service: swarm ↔ SDK, device registry, routing by `tool_id`
- [ ] Health/metrics endpoint, structured logging, config
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
- [ ] Choose Ed25519 library that passes licence policy and runs `no_std` (e.g. ed25519-dalek BSD-3)
- [ ] Key provisioning, storage, rotation and revocation design
- [ ] Replay protection (nonces/timestamps/counters)
- [ ] Implement handshake state machine on tpt-edge (ESP32 + STM32)
- [ ] Relay driver with hardware-enforced default-open
- [ ] Link-loss detection → relay open in < 1 ms
- [ ] Host-side handshake implementation in SDK
- [ ] Threat model document

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
- [ ] Define scope of the soft-body/stiffness/mass model (what fidelity is "enough")
- [ ] Choose implementation language and licence-clean dependencies (math/linear algebra)
- [ ] Hand/finger model with joints, stiffness, mass
- [ ] Apply `MorphogenesisAck` effects: stiffness/mass change on heal, tendon thickening on reinforce
- [ ] Healing timer driven by simulated time
- [ ] Injury injection API (tear a pathway)
- [ ] Simple grasp/lift task environment
- [ ] Integrate with `SimulatedHand` and the SDK over protobuf
- [ ] Validation against analytic/simple test cases
- [ ] Optional visualisation (licence-checked)

### Demo
- [ ] Agent "injures" a finger, waits for healing timer, completes the task
- [ ] Scripted baseline agent (no dependency on tpt-cerebrum) plus optional cerebrum-driven run
- [ ] Record demo and write walkthrough

---

## Phase 5 — Release Hardening to v1.0

### Docs
- [ ] `docs/ADDING_A_TOOL.md`
- [ ] API docs complete and published
- [ ] Architecture overview, wire protocol reference, power handshake spec
- [ ] Firmware porting guide (adding another MCU)
- [ ] Wiring/bring-up guide for ESP32 and STM32 dev boards (no CAD)

### Quality
- [ ] Test coverage review; add missing tests
- [ ] Fuzz targets for frame parsing and handshake messages
- [ ] Soak test: long-running link with fault injection
- [ ] Performance benchmarks: reflex loop latency, link throughput
- [ ] Final licence audit (all crates + Go modules + vendored protos) and `THIRD_PARTY_LICENSES`
- [ ] Semver and release process, CHANGELOG
- [ ] Publish all crates; tag v1.0

### v1.0 success criteria checklist
- [ ] New end-effector implementable in < 200 lines of Rust
- [ ] Simulated hand: tear → soma notified → chrysalis repair → recovery, no human intervention
- [ ] Power handshake formally verified, and fault-injection test passes (relay opens in < 1 ms)
- [ ] `tpt-hands-chrysalis-adapter` works against the real tpt-chrysalis repo API

---

## Open items to revisit
- [ ] Confirm protobuf/gRPC stack passes the Apache-only rule (blocking for Phase 1; see Phase 0 spikes)
- [ ] Decide CAN vs CAN-FD (STM32G4 has FDCAN; ESP32-C6 is classic CAN only, so likely classic CAN as baseline, FD optional)
- [ ] Decide whether `no_std` core types live in their own crate
