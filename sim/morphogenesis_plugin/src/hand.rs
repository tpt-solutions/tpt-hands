// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! `SimHand`: a `Finger` behind the SDK's `EndEffector` trait, so the same
//! control code can drive this model or the mock `SimulatedHand`.

use std::collections::VecDeque;

use tpt_hands_sdk::{
    ControlError, EndEffector, PathwayHealth, PathwayId, PathwayState, PhysiologicalState,
    ToolCapabilities, ToolIdentity, ToolIntent, ToolTelemetry,
};

use crate::{Finger, FingerParams};

/// Telemetry samples kept before the oldest are dropped.
const TELEMETRY_CAPACITY: usize = 1024;

/// A single-finger end-effector backed by the in-house model.
#[derive(Debug, Clone)]
pub struct SimHand {
    identity: ToolIdentity,
    capabilities: ToolCapabilities,
    finger: Finger,
    pending_reinforcement_n: Option<f32>,
    telemetry: VecDeque<ToolTelemetry>,
}

impl SimHand {
    /// Creates a finger with the given limits and physical parameters.
    pub fn new(identity: ToolIdentity, max_force_n: f32, params: FingerParams) -> Self {
        Self {
            identity,
            capabilities: ToolCapabilities::new(max_force_n, 1),
            finger: Finger::new(params),
            pending_reinforcement_n: None,
            telemetry: VecDeque::new(),
        }
    }

    /// The underlying model, for placing objects, injuring, and inspecting state.
    pub fn finger(&self) -> &Finger {
        &self.finger
    }

    /// Mutable access to the model, for placing objects and injuring.
    pub fn finger_mut(&mut self) -> &mut Finger {
        &mut self.finger
    }

    /// Advances the model by `dt` seconds and records one telemetry sample.
    pub fn tick(&mut self, dt: f64) {
        self.finger.step(dt);
        if self.telemetry.len() == TELEMETRY_CAPACITY {
            self.telemetry.pop_front();
        }
        self.telemetry.push_back(ToolTelemetry {
            timestamp_us: (self.finger.time_s() * 1.0e6) as u64,
            force_n: self.finger.contact_force_n() as f32,
            structural_integrity: self.finger.integrity() as f32,
        });
    }

    /// Completes a reinforcement the caller has confirmed, as a `ReinforceTendon`
    /// acknowledgement does. The stiffness rises by `volume_ml`, and the new
    /// force limit is not applied here. A `ReinforceTendon` ack only confirms
    /// the material is in place.
    pub fn complete_reinforcement(&mut self, volume_ml: f64) -> Result<(), ControlError> {
        if self.pending_reinforcement_n.take().is_none() {
            return Err(ControlError::NoPendingReinforcement);
        }
        self.finger
            .reinforce(volume_ml)
            .map_err(|_| ControlError::InvalidIntent {
                reason: "volume must be finite and non-negative",
            })
    }
}

impl EndEffector for SimHand {
    fn identity(&self) -> &ToolIdentity {
        &self.identity
    }

    fn capabilities(&self) -> &ToolCapabilities {
        &self.capabilities
    }

    fn command(&mut self, intent: ToolIntent) -> Result<(), ControlError> {
        match intent {
            ToolIntent::Grasp { force_n } => {
                if !force_n.is_finite() || force_n < 0.0 {
                    return Err(ControlError::InvalidIntent {
                        reason: "force must be finite and non-negative",
                    });
                }
                if force_n > self.capabilities.max_force_n {
                    return Err(ControlError::ExceedsCapability {
                        requested_n: force_n,
                        max_n: self.capabilities.max_force_n,
                    });
                }
                self.finger.set_grip_force(f64::from(force_n)).map_err(|_| {
                    ControlError::InvalidIntent {
                        reason: "force must be finite and non-negative",
                    }
                })
            }
            ToolIntent::Release => {
                self.finger
                    .set_grip_force(0.0)
                    .map_err(|_| ControlError::InvalidIntent {
                        reason: "release failed",
                    })
            }
            ToolIntent::PrepareForReinforcement { target_force_n } => {
                if !target_force_n.is_finite() || target_force_n <= 0.0 {
                    return Err(ControlError::InvalidIntent {
                        reason: "target force must be finite and positive",
                    });
                }
                self.pending_reinforcement_n = Some(target_force_n);
                Ok(())
            }
        }
    }

    fn next_telemetry(&mut self) -> Option<ToolTelemetry> {
        self.telemetry.pop_front()
    }

    fn physiological_state(&self) -> PhysiologicalState {
        let integrity = self.finger.integrity();
        let healing_time_ms = self.finger_healing_time_ms();
        let severed = integrity < 1.0;
        let state = if severed {
            PathwayState::Severed {
                healing_progress: integrity as f32,
            }
        } else {
            PathwayState::Intact
        };
        PhysiologicalState {
            pathways: vec![PathwayHealth {
                id: PathwayId(0),
                state,
                resistance_ohms: if severed { 500.0 } else { 10.0 },
                max_bandwidth_mbps: 100.0,
            }],
            structural_integrity: integrity as f32,
            estimated_recovery_time_ms: if severed {
                ((1.0 - integrity) * healing_time_ms) as u64
            } else {
                0
            },
            active_healing_protocols: if severed {
                vec![tpt_hands_sdk::HealingProtocolId(0)]
            } else {
                Vec::new()
            },
        }
    }
}

impl SimHand {
    fn finger_healing_time_ms(&self) -> f64 {
        self.finger.healing_time_s() * 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hand() -> SimHand {
        SimHand::new(ToolIdentity::new("hand-0"), 5.0, FingerParams::default())
    }

    #[test]
    fn grasp_within_capacity_sets_the_grip() {
        let mut h = hand();
        h.command(ToolIntent::Grasp { force_n: 2.0 }).unwrap();
        assert!((h.finger().grip_force_n() - 2.0).abs() < 1e-6);
    }

    #[test]
    fn grasp_over_capacity_is_refused_and_leaves_the_grip_alone() {
        let mut h = hand();
        h.command(ToolIntent::Grasp { force_n: 2.0 }).unwrap();
        let err = h.command(ToolIntent::Grasp { force_n: 9.0 }).unwrap_err();
        assert_eq!(
            err,
            ControlError::ExceedsCapability {
                requested_n: 9.0,
                max_n: 5.0
            }
        );
        assert!((h.finger().grip_force_n() - 2.0).abs() < 1e-6);
    }

    #[test]
    fn invalid_force_is_refused() {
        let mut h = hand();
        assert!(matches!(
            h.command(ToolIntent::Grasp { force_n: f32::NAN }),
            Err(ControlError::InvalidIntent { .. })
        ));
        assert!(matches!(
            h.command(ToolIntent::Grasp { force_n: -1.0 }),
            Err(ControlError::InvalidIntent { .. })
        ));
    }

    #[test]
    fn release_opens_the_grip() {
        let mut h = hand();
        h.command(ToolIntent::Grasp { force_n: 2.0 }).unwrap();
        h.command(ToolIntent::Release).unwrap();
        assert_eq!(h.finger().grip_force_n(), 0.0);
    }

    #[test]
    fn tick_records_telemetry_with_model_time() {
        let mut h = hand();
        h.tick(0.001);
        h.tick(0.001);
        let first = h.next_telemetry().unwrap();
        assert_eq!(first.timestamp_us, 1_000);
        let second = h.next_telemetry().unwrap();
        assert_eq!(second.timestamp_us, 2_000);
        assert!(h.next_telemetry().is_none());
    }

    #[test]
    fn severed_finger_reports_a_healing_pathway() {
        let mut h = hand();
        h.finger_mut().injure();
        let state = h.physiological_state();
        assert_eq!(state.pathways.len(), 1);
        assert!(matches!(
            state.pathways[0].state,
            PathwayState::Severed { .. }
        ));
        assert_eq!(state.active_healing_protocols.len(), 1);
        assert!(state.structural_integrity < 1.0);
        assert_eq!(state.estimated_recovery_time_ms, 2000);
    }

    #[test]
    fn intact_finger_reports_no_healing() {
        let state = hand().physiological_state();
        assert_eq!(state.structural_integrity, 1.0);
        assert_eq!(state.estimated_recovery_time_ms, 0);
        assert!(state.active_healing_protocols.is_empty());
    }

    #[test]
    fn reinforcement_must_be_prepared_first() {
        let mut h = hand();
        assert_eq!(
            h.complete_reinforcement(1.0),
            Err(ControlError::NoPendingReinforcement)
        );
        h.command(ToolIntent::PrepareForReinforcement {
            target_force_n: 4.0,
        })
        .unwrap();
        let before = h.finger().stiffness_nm_per_rad();
        h.complete_reinforcement(1.0).unwrap();
        assert!(h.finger().stiffness_nm_per_rad() > before);
    }

    #[test]
    fn works_through_the_trait_object() {
        // Control code written against the SDK trait runs unchanged on this model.
        let mut end: Box<dyn EndEffector> = Box::new(hand());
        end.command(ToolIntent::Grasp { force_n: 1.0 }).unwrap();
        assert_eq!(end.capabilities().max_force_n, 5.0);
        assert_eq!(end.identity().tool_id, "hand-0");
    }
}
