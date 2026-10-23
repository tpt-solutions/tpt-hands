tpt-hands — Design Specification (v0.2.0)
Version: 0.2.0 (Ecosystem Integrated)
Status: Proposal
Date: 2026-10-12
Author: TPT
Stack: Rust (edge + SDK) · Go (orchestration) · Protobuf (wire) · ESP32-C6 + STM32G4 (reference edge nodes)
Licence: MIT OR Apache-2.0 (dual), copyright TPT Solutions. See todo.md for the dependency licence policy.
1. Vision
tpt-hands is the peripheral nervous system and morphological edge layer for the TPT robotics ecosystem. It is not merely a driver; it is the real-time, hardware-agnostic bridge that connects high-level cognitive intent (tpt-cerebrum), whole-body physiological state tracking (tpt-soma), and formally verified microfluidic actuation (tpt-chrysalis) to physical, self-healing, and morphing end-effectors.
By leveraging existing TPT infrastructure, tpt-hands enables robotic endpoints to exhibit biological behaviors: local reflexes, graceful degradation, self-healing of torn pathways, and dynamic morphogenesis (growing new data/power conduits or structural tendons on demand).
1.1 Goals
Ecosystem Synergy: Act as the physical execution layer for tpt-cerebrum, tpt-soma, and tpt-chrysalis.
Bio-mimetic Morphology: Support self-healing elastomers and liquid-metal microfluidics via formally verified control.
Hardware Agnosticism: One SDK drives any endpoint (dexterous hand, gripper, spindle, extruder).
Solo-Maintainer Sustainable: Focus purely on protocols, traits, and adapters. Leverage tpt-chrysalis for complex microfluidic logic; do not reinvent it.
1.2 Non-Goals
Designing specific mechanical CAD models (out of scope; community/hardware partners build these).
Replacing tpt-chrysalis’s microfluidic orchestration (we adapt to it, we don't fork it).
Building general-purpose AI cognition (that is tpt-cerebrum).
Hardware design of any kind: no CAD models, no reference hardware design. This project is software only.
2. Core Principles
#
Principle
Meaning
P1
The Spinal Cord, Not the Brain
tpt-hands handles µs/ms local reflexes and state reporting. High-level planning belongs to tpt-cerebrum.
P2
Digital Twin Fidelity
Every physical state change (tear, healing, growth) is instantly mirrored in tpt-soma.
P3
Formally Verified Morphogenesis
Physical "growth" (pumping fluids) is strictly delegated to tpt-chrysalis to guarantee safety invariants.
P4
Fail-Soft, Never Fail-Hard
A broken pathway redistributes load locally; it does not crash the system.
P5
Power is a Protocol
No high-power relay closes without a cryptographically signed handshake.
3. System Architecture
┌──────────────────────────────────────────────────────────────────────┐
│  COGNITIVE & PHYSIOLOGICAL LAYER (Host / Cloud)                      │
│                                                                      │
│  ┌────────────────┐      ┌────────────────┐      ┌────────────────┐  │
│  │ tpt-cerebrum   │─────▶│  tpt-soma      │─────▶│ tpt-chrysalis  │  │
│  │ (Intent/Plan)  │      │ (Digital Twin) │      │ (Microfluidics)│  │
│  └────────────────┘      └───────┬────────┘      └───────┬────────┘  │
│                                  │                       │           │
│                                  ▼                       ▼           │
│                       ┌──────────────────────────────────────────┐   │
│                       │       tpt-hands-sdk (Rust/Go)            │   │
│                       │  - EndEffector Traits                    │   │
│                       │  - Soma Sync Adapter                     │   │
│                       │  - Chrysalis Microfluidic Adapter        │   │
│                       └────────────────────┬─────────────────────┘   │
└───────────────────────────────────────────┼─────────────────────────┘
                                            │ tpt-link (Protobuf over CAN/Serial/UDP)
┌───────────────────────────────────────────┼─────────────────────────┐
│  PERIPHERAL EDGE LAYER (ESP32 / STM32 on Tool)                      │
│                                            │                        │
│                       ┌────────────────────▼─────────────────────┐   │
│                       │       tpt-edge firmware (Rust/esp-rs)    │   │
│                       │  - Local Reflex Loops (1kHz)             │   │
│                       │  - Smart Power Handshake                 │   │
│                       │  - Pathway Health Monitoring             │   │
│                       └────────────────────┬─────────────────────┘   │
│                                            │                        │
│                       ┌────────────────────▼─────────────────────┐   │
│                       │       Physical Morphology                │   │
│                       │  - Liquid Metal Pathways (Data/Power)    │   │
│                       │  - Hydrogel Tendons (Actuation)          │   │
│                       │  - Micro-valves & Reservoirs             │   │
│                       └──────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────┘

4. Core Abstractions (Rust)
4.1 The EndEffector Trait
The single entry point for all manipulation logic.

pub trait EndEffector: Send + Sync {
    fn capabilities(&self) -> &ToolCapabilities;
    fn command(&mut self, intent: ToolIntent) -> Result<(), ControlError>;
    fn telemetry(&self) -> Receiver<ToolTelemetry>; // Receiver type TBD: runtime-agnostic stream (todo.md, Phase 0)
    
    /// Returns current physiological state for tpt-soma sync
    fn physiological_state(&self) -> PhysiologicalState;
}

4.2 PhysiologicalState (The tpt-soma Bridge)
Tracks the "health" of the synthetic biology.

pub struct PhysiologicalState {
    pub pathways: Vec<PathwayHealth>,
    pub structural_integrity: f32, // 0.0 to 1.0
    pub estimated_recovery_time_ms: u64,
    pub active_healing_protocols: Vec<HealingProtocolId>,
}

pub struct PathwayHealth {
    pub id: PathwayId,
    pub state: PathwayState, // Intact, Severed(healing_progress), Hypertrophied
    pub resistance_ohms: f32,
    pub max_bandwidth_mbps: f32,
}

4.3 MorphogenesisRequest (The tpt-chrysalis Bridge)
When the edge detects a need for physical repair or growth, it does not actuate pumps directly. It sends a formally verified request to tpt-chrysalis.

pub struct MorphogenesisRequest {
    pub target_endpoint: ToolIdentity,
    pub action: MorphAction,
    pub safety_envelope: SafetyEnvelope, // Formally verified constraints
}

pub enum MorphAction {
    HealPathway { pathway_id: PathwayId, heat_assist: bool },
    SproutConduit { origin: ChannelId, destination: ChannelId, fluid_type: FluidType },
    ReinforceTendon { tendon_id: TendonId, volume_ml: f32 },
}

5. Bio-Mimetic Workflows
5.1 The "Healing" Loop (Tear & Repair)
Detection: tpt-edge (ESP32) detects a spike in resistance on Pathway A during a grasp.
Local Reflex: tpt-edge instantly reroutes critical telemetry to Pathway B and downgrades Pathway A to Severed { healing_progress: 0.0 }.
State Sync: tpt-hands-sdk streams a PhysiologicalState update to tpt-soma. The digital twin logs the "injury."
Repair Orchestration: tpt-soma calculates the required repair and emits a MorphogenesisRequest::HealPathway to tpt-chrysalis.
Formal Execution: tpt-chrysalis verifies the request against its safety invariants, then actuates the physical micro-valves and heaters to inject self-healing elastomer precursor and apply mild heat.
Recovery: tpt-edge monitors the dropping resistance. Once healing_progress >= 1.0, it reports Intact to tpt-soma.
5.2 The "Growing" Loop (Dynamic Morphogenesis)
Trigger: tpt-cerebrum plans a heavy lift requiring 50N of force, but tpt-soma reports the hand's current max capacity is 30N.
Adaptation: tpt-cerebrum commands tpt-hands to prepare for reinforcement.
Request: tpt-hands sends MorphogenesisRequest::ReinforceTendon to tpt-chrysalis.
Execution: tpt-chrysalis pumps fast-curing hydrogel into dormant micro-channels within the finger structure, physically thickening the tendon.
Recalibration: tpt-edge runs a 500ms micro-calibration cycle to map the new physical limits, updates ToolCapabilities, and reports success. The lift proceeds.
6. Wire Protocol (tpt-link) Extensions
Additions to the Protobuf schema to support the bio-mimetic stack.
Codegen: Rust via micropb / micropb-gen (std and no_std); Go via google.golang.org/protobuf. No prost anywhere in the tree (Apache-2.0 only).

message PhysiologicalUpdate {
    string tool_id = 1;
    float structural_integrity = 2;
    repeated PathwayHealth pathways = 3;
    uint64 timestamp_us = 4;
}

message MorphogenesisRequest {
    string tool_id = 1;
    oneof action {
        HealPathwayAction heal = 10;
        SproutConduitAction sprout = 11;
        ReinforceTendonAction reinforce = 12;
    }
    SafetyEnvelope safety_limits = 99;
}

message MorphogenesisAck {
    string request_id = 1;
    bool approved = 2;
    string chrysalis_job_id = 3; // Tracked by tpt-chrysalis
}

7. Smart Power Handshake (Unchanged but Critical)
The two-stage, cryptographically signed power negotiation (Data pins mate first → Ed25519 handshake → Relay closure) remains a core, non-negotiable feature of tpt-edge to ensure safe interaction with ubiquitous power sources.
8. Repository Layout & Ecosystem Mapping

tpt-hands/
├── DESIGN.md                 ← This document
├── Cargo.toml                ← Workspace root
├── crates/
│   ├── tpt-hands-sdk/        ← Public Rust SDK (traits, mocks, drivers)
│   ├── tpt-hands-proto/      ← Protobuf definitions (shared with tpt-link)
│   ├── tpt-hands-edge/       ← ESP32/STM32 firmware (esp-rs / embassy)
│   ├── tpt-hands-soma-adapter/  ← Syncs PhysiologicalState to tpt-soma
│   └── tpt-hands-chrysalis-adapter/ ← Sends MorphogenesisRequests to tpt-chrysalis
├── go/
│   └── tpt-hands-orchestrator/  ← Go service bridging tpt-swarm and the SDK
├── sim/
│   └── morphogenesis_plugin/ ← In-house minimal soft-body model (stiffness, mass, healing timer)
├── examples/
│   ├── sim_healing_loop/     ← End-to-end mock of tear → soma update → chrysalis repair
│   └── smart_socket_bringup/ ← Power handshake demo
└── docs/
    ├── ECOSYSTEM_INTEGRATION.md ← How to connect to tpt-soma/tpt-chrysalis
    └── ADDING_A_TOOL.md

9. Phase Roadmap (Revised)
Phase 0 — Foundation & Traits (Weeks 1–2)
Define EndEffector, PhysiologicalState, and MorphogenesisRequest traits.
Ship SimulatedHand mock driver with fake healing/growth state machines.
Publish tpt-hands-sdk crate (v0.1).
Phase 1 — Edge Reflexes & Protocol (Weeks 3–5)
Implement tpt-edge on ESP32: local slip/overload reflexes, resistance monitoring.
Implement tpt-link transport (Serial/USB-CDC for dev).
End-to-end loopback: Simulated tear detected by edge, reported via telemetry.
Phase 2 — Ecosystem Adapters (Weeks 6–9)
Build tpt-hands-soma-adapter: Stream PhysiologicalState to a mock tpt-soma endpoint.
Build tpt-hands-chrysalis-adapter: Format and send MorphogenesisRequest to a mock tpt-chrysalis endpoint.
Write integration tests proving the full loop: Tear → Soma Update → Chrysalis Request → Ack.
Phase 3 — Smart Power & Formal Proofs (Weeks 10–13)
Implement Ed25519 signed power handshake on tpt-edge.
Add Kani/TLA+ proofs for power invariants (relay_closed ⇒ data_link_alive).
CLI simulator for handshake fault injection.
Phase 4 — Sim2Real Morphogenesis (Weeks 14–16)
Develop the in-house simulation model (no MuJoCo/Genesis) that alters joint stiffness/mass when a MorphogenesisAck is received.
Demo: AI agent in simulation "injures" a finger, waits for the simulated healing timer, and successfully completes the task.
10. Success Criteria
tpt-hands is "done" (v1.0) when:
A developer can implement a new end-effector in < 200 lines of Rust.
A simulated hand can experience a "torn pathway," automatically notify tpt-soma, trigger a tpt-chrysalis repair job, and recover functionality without human intervention.
The smart power handshake passes formal verification and a physical fault-injection test (cable yanked → relay opens in < 1 ms).
The tpt-hands-chrysalis-adapter successfully communicates with the actual tpt-chrysalis repository's API.
11. Decisions (resolved)
Adapter Coupling: gRPC/Protobuf only. tpt-hands takes no crate dependency on tpt-soma, tpt-chrysalis or the other ecosystem repos.
Simulation Fidelity: in-house minimal soft-body model. No MuJoCo, no Genesis, no wrapped physics library.
Hardware Reference: software only. No CAD, no 3D-printable reference design.
Go Orchestrator: included in v1.0 as a thin service.
MCUs: ESP32-C6 and STM32G4 (FDCAN, embassy-stm32) as reference targets. Transports: Serial/USB-CDC, UDP, CAN.

12. Open Items
CAN vs CAN-FD: classic CAN is the baseline. FD is optional; ESP32-C6 supports classic CAN only.
no_std core types: whether they live in their own crate.
Telemetry channel type: runtime-agnostic stream to replace Receiver in EndEffector.