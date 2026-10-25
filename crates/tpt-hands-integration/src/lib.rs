// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! End-to-end mock scenarios. A simulated hand, the edge reflexes, and the
//! chrysalis adapter run together, with the chrysalis side served by a MOCK.
//!
//! Known limit: the simulated hand heals on its own timer and does not depend on
//! chrysalis. An approved repair is recorded, but it does not speed up or change
//! the healing. That is a property of the model, not a result. The scenarios say
//! so where it matters.

use std::sync::Arc;

use tpt_hands_edge::reflex::{PathwayEvent, PathwayMonitor, pick_route};
use tpt_hands_sdk::{
    EndEffector, ManualClock, MorphAction, MorphogenesisRequest, PathwayId, SafetyEnvelope,
    SimulatedHand, ToolCapabilities, ToolIdentity,
};

/// The edge's sever threshold, between the sim's intact (10 ohm) and severed
/// (500 ohm) resistances.
pub const SEVER_OHMS: f32 = 100.0;
/// The edge's recover threshold. It is below the sever threshold, so the state
/// does not chatter.
pub const RECOVER_OHMS: f32 = 50.0;
/// Consecutive samples needed before the edge changes state.
pub const DEBOUNCE: u8 = 2;

/// One simulated hand with an edge monitor per pathway.
pub struct Rig {
    pub hand: SimulatedHand,
    pub clock: Arc<ManualClock>,
    pub monitors: Vec<PathwayMonitor>,
}

impl Rig {
    /// A hand with two pathways, both intact. Pathway 0 is preferred.
    pub fn new() -> Self {
        let clock = Arc::new(ManualClock::new(0));
        let hand = SimulatedHand::new(
            ToolIdentity::new("hand-0"),
            ToolCapabilities::new(30.0, 5),
            vec![PathwayId(0), PathwayId(1)],
            clock.clone(),
        );
        Self {
            hand,
            clock,
            monitors: vec![PathwayMonitor::new(SEVER_OHMS, RECOVER_OHMS, DEBOUNCE); 2],
        }
    }

    /// Feeds each monitor the hand's current resistance and returns what changed.
    pub fn sample(&mut self) -> Vec<(usize, PathwayEvent)> {
        let state = self.hand.physiological_state();
        let mut events = Vec::new();
        for (i, pathway) in state.pathways.iter().enumerate() {
            let event = self.monitors[i].sample(pathway.resistance_ohms);
            if event != PathwayEvent::None {
                events.push((i, event));
            }
        }
        events
    }

    /// Advances time by `ms`, runs the hand, and samples the edge.
    pub fn step(&mut self, ms: u64) -> Vec<(usize, PathwayEvent)> {
        self.clock.advance_us(ms * 1_000);
        self.hand.tick();
        self.sample()
    }

    /// Where telemetry goes right now, preferring pathway 0.
    pub fn route(&self) -> Option<usize> {
        pick_route(&self.monitors, &[0, 1])
    }
}

impl Default for Rig {
    fn default() -> Self {
        Self::new()
    }
}

/// The repair request tpt-hands sends to chrysalis for a torn pathway.
pub fn repair_request(pathway: u16) -> MorphogenesisRequest {
    MorphogenesisRequest {
        target_endpoint: ToolIdentity::new("hand-0"),
        action: MorphAction::HealPathway {
            pathway_id: PathwayId(pathway),
            heat_assist: false,
        },
        safety_envelope: SafetyEnvelope {
            max_pressure_kpa: 200.0,
            max_temperature_c: 60.0,
            max_volume_ml: 5.0,
        },
    }
}
