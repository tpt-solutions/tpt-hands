// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! A complete new end-effector, to show how little code a driver needs.
//!
//! This is a two-jaw gripper with no pathways. It implements `EndEffector`
//! and nothing else. Run it with `cargo run -p tpt-hands-sdk --example new_end_effector`.

use std::collections::VecDeque;

use tpt_hands_sdk::{
    ControlError, EndEffector, PhysiologicalState, ToolCapabilities, ToolIdentity, ToolIntent,
    ToolTelemetry,
};

/// A gripper driven by a pneumatic jaw with a fixed force limit.
struct Gripper {
    identity: ToolIdentity,
    capabilities: ToolCapabilities,
    grip_force_n: f32,
    time_us: u64,
    telemetry: VecDeque<ToolTelemetry>,
}

impl Gripper {
    fn new(max_force_n: f32) -> Self {
        Self {
            identity: ToolIdentity::new("gripper-0"),
            capabilities: ToolCapabilities::new(max_force_n, 1),
            grip_force_n: 0.0,
            time_us: 0,
            telemetry: VecDeque::new(),
        }
    }

    /// Stands in for the hardware sample loop (called from a timer).
    fn sample(&mut self, dt_us: u64) {
        self.time_us += dt_us;
        self.telemetry.push_back(ToolTelemetry {
            timestamp_us: self.time_us,
            force_n: self.grip_force_n,
            structural_integrity: 1.0,
        });
    }
}

impl EndEffector for Gripper {
    fn identity(&self) -> &ToolIdentity {
        &self.identity
    }

    fn capabilities(&self) -> &ToolCapabilities {
        &self.capabilities
    }

    fn command(&mut self, intent: ToolIntent) -> Result<(), ControlError> {
        match intent {
            ToolIntent::Grasp { force_n } if !force_n.is_finite() || force_n < 0.0 => {
                Err(ControlError::InvalidIntent {
                    reason: "force must be finite and non-negative",
                })
            }
            ToolIntent::Grasp { force_n } if force_n > self.capabilities.max_force_n => {
                Err(ControlError::ExceedsCapability {
                    requested_n: force_n,
                    max_n: self.capabilities.max_force_n,
                })
            }
            ToolIntent::Grasp { force_n } => {
                self.grip_force_n = force_n;
                Ok(())
            }
            ToolIntent::Release => {
                self.grip_force_n = 0.0;
                Ok(())
            }
            // A gripper cannot thicken its own jaws, so it has nothing to prepare.
            ToolIntent::PrepareForReinforcement { .. } => Err(ControlError::InvalidIntent {
                reason: "gripper has no reinforceable tendons",
            }),
        }
    }

    fn next_telemetry(&mut self) -> Option<ToolTelemetry> {
        self.telemetry.pop_front()
    }

    fn physiological_state(&self) -> PhysiologicalState {
        PhysiologicalState {
            pathways: Vec::new(),
            structural_integrity: 1.0,
            estimated_recovery_time_ms: 0,
            active_healing_protocols: Vec::new(),
        }
    }
}

fn main() -> Result<(), ControlError> {
    let mut gripper = Gripper::new(20.0);
    gripper.command(ToolIntent::Grasp { force_n: 12.0 })?;
    gripper.sample(1_000);

    println!(
        "{} capacity {} N",
        gripper.identity().tool_id,
        gripper.capabilities().max_force_n
    );
    while let Some(sample) = gripper.next_telemetry() {
        println!("t={}us force={} N", sample.timestamp_us, sample.force_n);
    }
    Ok(())
}
