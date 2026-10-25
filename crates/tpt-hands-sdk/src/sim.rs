// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! `SimulatedHand`, a mock end-effector with fake healing and growth state
//! machines, and the clocks it runs on.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::end_effector::{ControlError, EndEffector, ToolIntent, ToolTelemetry};
use crate::types::{
    HealingProtocolId, PathwayHealth, PathwayId, PathwayState, PhysiologicalState,
    ToolCapabilities, ToolIdentity,
};

/// Time for a severed pathway to heal, in microseconds (2 s).
const HEALING_TIME_US: u64 = 2_000_000;
/// Micro-calibration time after reinforcement, in microseconds (500 ms).
const RECALIBRATION_US: u64 = 500_000;
/// Resistance of an intact pathway, in ohms.
const INTACT_RESISTANCE_OHMS: f32 = 10.0;
/// Resistance of a severed pathway, in ohms.
const SEVERED_RESISTANCE_OHMS: f32 = 500.0;
/// Bandwidth of a pathway, in Mbps.
const PATHWAY_BANDWIDTH_MBPS: f32 = 100.0;
/// Telemetry samples kept before the oldest are dropped.
const TELEMETRY_CAPACITY: usize = 1024;

/// A source of time, in microseconds. Inject a `ManualClock` in tests so that
/// healing and recalibration are deterministic.
pub trait Clock: Send + Sync {
    /// Current time in microseconds.
    fn now_us(&self) -> u64;
}

/// A clock that only moves when told to. Use it in tests.
///
/// ```
/// use tpt_hands_sdk::{Clock, ManualClock};
///
/// let clock = ManualClock::new(100);
/// clock.advance_us(50);
/// assert_eq!(clock.now_us(), 150);
/// ```
#[derive(Debug, Default)]
pub struct ManualClock {
    now_us: AtomicU64,
}

impl ManualClock {
    /// Creates a clock starting at `start_us`.
    pub fn new(start_us: u64) -> Self {
        Self {
            now_us: AtomicU64::new(start_us),
        }
    }

    /// Moves the clock forward by `dt_us`.
    pub fn advance_us(&self, dt_us: u64) {
        self.now_us.fetch_add(dt_us, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now_us(&self) -> u64 {
        self.now_us.load(Ordering::SeqCst)
    }
}

/// A clock backed by the system's monotonic timer. Use it on real hardware.
#[derive(Debug)]
pub struct MonotonicClock {
    start: Instant,
}

impl MonotonicClock {
    /// Creates a clock that starts counting now.
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MonotonicClock {
    fn now_us(&self) -> u64 {
        u64::try_from(self.start.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}

/// A mock end-effector for tests and demos.
///
/// Its state machines are simple but real:
///
/// - **Healing:** `tear` severs a pathway. Each `tick` advances its
///   `healing_progress` by elapsed time, and it becomes `Intact` after 2 s.
/// - **Growth:** `command(PrepareForReinforcement)` records a target force.
///   After the chrysalis ack, `complete_reinforcement` starts a 500 ms
///   recalibration, and the new force limit applies when it finishes.
///
/// Time comes from the injected `Clock`. The hand only advances when `tick`
/// is called.
///
/// ```
/// use std::sync::Arc;
/// use tpt_hands_sdk::{
///     EndEffector, ManualClock, PathwayId, SimulatedHand, ToolCapabilities, ToolIdentity,
/// };
///
/// let clock = Arc::new(ManualClock::new(0));
/// let mut hand = SimulatedHand::new(
///     ToolIdentity::new("hand-0"),
///     ToolCapabilities::new(30.0, 5),
///     vec![PathwayId(0), PathwayId(1)],
///     clock.clone(),
/// );
///
/// hand.tear(PathwayId(0)).unwrap();
/// clock.advance_us(1_000_000);
/// hand.tick();
/// assert!(hand.physiological_state().structural_integrity < 1.0);
/// ```
pub struct SimulatedHand {
    identity: ToolIdentity,
    capabilities: ToolCapabilities,
    clock: Arc<dyn Clock>,
    last_tick_us: u64,
    pathways: Vec<PathwayHealth>,
    grasp_force_n: f32,
    pending_reinforcement_n: Option<f32>,
    recalibrating_to_n: Option<f32>,
    recalibration_left_us: u64,
    telemetry: VecDeque<ToolTelemetry>,
}

impl SimulatedHand {
    /// Creates an intact hand with the given pathways.
    pub fn new(
        identity: ToolIdentity,
        capabilities: ToolCapabilities,
        pathway_ids: Vec<PathwayId>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let last_tick_us = clock.now_us();
        let pathways = pathway_ids
            .into_iter()
            .map(|id| PathwayHealth {
                id,
                state: PathwayState::Intact,
                resistance_ohms: INTACT_RESISTANCE_OHMS,
                max_bandwidth_mbps: PATHWAY_BANDWIDTH_MBPS,
            })
            .collect();
        Self {
            identity,
            capabilities,
            clock,
            last_tick_us,
            pathways,
            grasp_force_n: 0.0,
            pending_reinforcement_n: None,
            recalibrating_to_n: None,
            recalibration_left_us: 0,
            telemetry: VecDeque::new(),
        }
    }

    /// Injects a tear: the pathway becomes `Severed` with no healing progress.
    pub fn tear(&mut self, id: PathwayId) -> Result<(), ControlError> {
        let pathway = self.pathway_mut(id)?;
        pathway.state = PathwayState::Severed {
            healing_progress: 0.0,
        };
        pathway.resistance_ohms = SEVERED_RESISTANCE_OHMS;
        Ok(())
    }

    /// Marks a pathway as thickened by reinforcement.
    pub fn hypertrophy(&mut self, id: PathwayId) -> Result<(), ControlError> {
        let pathway = self.pathway_mut(id)?;
        pathway.state = PathwayState::Hypertrophied;
        Ok(())
    }

    /// Records that the chrysalis reinforcement was applied. This starts the
    /// recalibration, and the new force limit takes effect when it finishes.
    pub fn complete_reinforcement(&mut self) -> Result<(), ControlError> {
        let target = self
            .pending_reinforcement_n
            .take()
            .ok_or(ControlError::NoPendingReinforcement)?;
        self.recalibrating_to_n = Some(target);
        self.recalibration_left_us = RECALIBRATION_US;
        Ok(())
    }

    /// Advances the state machines to the clock's current time, and records a
    /// telemetry sample.
    pub fn tick(&mut self) {
        let now = self.clock.now_us();
        let dt = now.saturating_sub(self.last_tick_us);
        self.last_tick_us = now;

        for pathway in &mut self.pathways {
            let healed = match &mut pathway.state {
                PathwayState::Severed { healing_progress } => {
                    *healing_progress =
                        (*healing_progress + dt as f32 / HEALING_TIME_US as f32).min(1.0);
                    *healing_progress >= 1.0
                }
                _ => false,
            };
            if healed {
                pathway.state = PathwayState::Intact;
                pathway.resistance_ohms = INTACT_RESISTANCE_OHMS;
            }
        }

        if self.recalibration_left_us > 0 {
            self.recalibration_left_us = self.recalibration_left_us.saturating_sub(dt);
            if self.recalibration_left_us == 0
                && let Some(max_n) = self.recalibrating_to_n.take()
            {
                self.capabilities.max_force_n = max_n;
            }
        }

        if self.telemetry.len() == TELEMETRY_CAPACITY {
            self.telemetry.pop_front();
        }
        self.telemetry.push_back(ToolTelemetry {
            timestamp_us: now,
            force_n: self.grasp_force_n,
            structural_integrity: self.structural_integrity(),
        });
    }

    fn pathway_mut(&mut self, id: PathwayId) -> Result<&mut PathwayHealth, ControlError> {
        self.pathways
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(ControlError::UnknownPathway(id))
    }

    fn structural_integrity(&self) -> f32 {
        if self.pathways.is_empty() {
            return 1.0;
        }
        let sum: f32 = self
            .pathways
            .iter()
            .map(|p| match p.state {
                PathwayState::Severed { healing_progress } => healing_progress,
                PathwayState::Intact | PathwayState::Hypertrophied => 1.0,
            })
            .sum();
        sum / self.pathways.len() as f32
    }
}

impl EndEffector for SimulatedHand {
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
                self.grasp_force_n = force_n;
                Ok(())
            }
            ToolIntent::Release => {
                self.grasp_force_n = 0.0;
                Ok(())
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
        let mut recovery_us = 0.0_f32;
        let mut active_healing_protocols = Vec::new();
        for pathway in &self.pathways {
            if let PathwayState::Severed { healing_progress } = pathway.state {
                recovery_us += (1.0 - healing_progress) * HEALING_TIME_US as f32;
                active_healing_protocols.push(HealingProtocolId(u32::from(pathway.id.0)));
            }
        }
        PhysiologicalState {
            pathways: self.pathways.clone(),
            structural_integrity: self.structural_integrity(),
            estimated_recovery_time_ms: (recovery_us / 1000.0) as u64,
            active_healing_protocols,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-3;

    fn hand(max_force_n: f32) -> (SimulatedHand, Arc<ManualClock>) {
        let clock = Arc::new(ManualClock::new(0));
        let hand = SimulatedHand::new(
            ToolIdentity::new("hand-test"),
            ToolCapabilities::new(max_force_n, 5),
            vec![PathwayId(0), PathwayId(1)],
            clock.clone(),
        );
        (hand, clock)
    }

    fn pathway_state(hand: &SimulatedHand, id: PathwayId) -> PathwayState {
        hand.physiological_state()
            .pathways
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.state)
            .unwrap()
    }

    #[test]
    fn tear_then_heal_to_intact() {
        let (mut hand, clock) = hand(30.0);
        hand.tear(PathwayId(0)).unwrap();

        clock.advance_us(HEALING_TIME_US / 2);
        hand.tick();
        match pathway_state(&hand, PathwayId(0)) {
            PathwayState::Severed { healing_progress } => {
                assert!((healing_progress - 0.5).abs() < EPS)
            }
            other => panic!("expected Severed, got {other:?}"),
        }

        clock.advance_us(HEALING_TIME_US / 2);
        hand.tick();
        assert_eq!(pathway_state(&hand, PathwayId(0)), PathwayState::Intact);
        assert!((hand.physiological_state().structural_integrity - 1.0).abs() < EPS);
    }

    #[test]
    fn severed_pathway_is_reported_with_recovery_time() {
        let (mut hand, _clock) = hand(30.0);
        hand.tear(PathwayId(1)).unwrap();

        let state = hand.physiological_state();
        assert_eq!(state.active_healing_protocols, vec![HealingProtocolId(1)]);
        assert_eq!(state.estimated_recovery_time_ms, 2000);
        assert!((state.structural_integrity - 0.5).abs() < EPS);
    }

    #[test]
    fn grasp_within_capacity_is_sampled_in_telemetry() {
        let (mut hand, clock) = hand(30.0);
        hand.command(ToolIntent::Grasp { force_n: 10.0 }).unwrap();
        clock.advance_us(1_000);
        hand.tick();

        let sample = hand.next_telemetry().unwrap();
        assert_eq!(sample.timestamp_us, 1_000);
        assert_eq!(sample.force_n, 10.0);
        assert!(hand.next_telemetry().is_none());
    }

    #[test]
    fn grasp_over_capacity_is_rejected() {
        let (mut hand, _clock) = hand(30.0);
        let err = hand
            .command(ToolIntent::Grasp { force_n: 50.0 })
            .unwrap_err();
        assert_eq!(
            err,
            ControlError::ExceedsCapability {
                requested_n: 50.0,
                max_n: 30.0
            }
        );
    }

    #[test]
    fn invalid_force_is_rejected() {
        let (mut hand, _clock) = hand(30.0);
        assert!(matches!(
            hand.command(ToolIntent::Grasp { force_n: f32::NAN }),
            Err(ControlError::InvalidIntent { .. })
        ));
        assert!(matches!(
            hand.command(ToolIntent::Grasp { force_n: -1.0 }),
            Err(ControlError::InvalidIntent { .. })
        ));
    }

    #[test]
    fn unknown_pathway_is_rejected() {
        let (mut hand, _clock) = hand(30.0);
        assert_eq!(
            hand.tear(PathwayId(9)),
            Err(ControlError::UnknownPathway(PathwayId(9)))
        );
    }

    #[test]
    fn reinforcement_raises_capability_after_recalibration() {
        let (mut hand, clock) = hand(30.0);
        hand.command(ToolIntent::PrepareForReinforcement {
            target_force_n: 50.0,
        })
        .unwrap();
        hand.complete_reinforcement().unwrap();

        // Still in recalibration, so the old limit applies.
        clock.advance_us(RECALIBRATION_US - 1);
        hand.tick();
        assert_eq!(hand.capabilities().max_force_n, 30.0);

        clock.advance_us(1);
        hand.tick();
        assert_eq!(hand.capabilities().max_force_n, 50.0);
        assert!(hand.command(ToolIntent::Grasp { force_n: 50.0 }).is_ok());
    }

    #[test]
    fn complete_without_prepare_is_rejected() {
        let (mut hand, _clock) = hand(30.0);
        assert_eq!(
            hand.complete_reinforcement(),
            Err(ControlError::NoPendingReinforcement)
        );
    }

    #[test]
    fn hypertrophy_marks_pathway() {
        let (mut hand, _clock) = hand(30.0);
        hand.hypertrophy(PathwayId(0)).unwrap();
        assert_eq!(
            pathway_state(&hand, PathwayId(0)),
            PathwayState::Hypertrophied
        );
    }
}
