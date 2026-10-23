// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! The `EndEffector` trait and the intent, telemetry and error types it uses.

use std::fmt;

use crate::types::{PathwayId, PhysiologicalState, ToolCapabilities, ToolIdentity};

/// A command from the cognitive layer (tpt-cerebrum) to an end-effector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolIntent {
    /// Close on an object with the given force.
    Grasp {
        /// Requested grip force in newtons.
        force_n: f32,
    },
    /// Open the grip.
    Release,
    /// Prepare for a heavier load. The end-effector reports what it needs, and
    /// the caller then sends a `ReinforceTendon` request to tpt-chrysalis.
    PrepareForReinforcement {
        /// Force the end-effector must be able to reach, in newtons.
        target_force_n: f32,
    },
}

/// One telemetry sample from an end-effector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolTelemetry {
    /// Sample time in microseconds, from the end-effector's clock.
    pub timestamp_us: u64,
    /// Current grip force in newtons.
    pub force_n: f32,
    /// Overall structural integrity, from 0.0 to 1.0.
    pub structural_integrity: f32,
}

/// Why an end-effector rejected a command.
#[derive(Debug, Clone, PartialEq)]
pub enum ControlError {
    /// The intent has invalid values, such as a NaN or negative force.
    InvalidIntent {
        /// Short reason for the rejection.
        reason: &'static str,
    },
    /// The requested force is more than the end-effector can produce.
    ExceedsCapability {
        /// Force that was requested, in newtons.
        requested_n: f32,
        /// Current maximum force, in newtons.
        max_n: f32,
    },
    /// No pathway with this id exists on the end-effector.
    UnknownPathway(PathwayId),
    /// A reinforcement was completed, but none was prepared.
    NoPendingReinforcement,
}

impl fmt::Display for ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIntent { reason } => write!(f, "invalid intent: {reason}"),
            Self::ExceedsCapability { requested_n, max_n } => {
                write!(
                    f,
                    "requested {requested_n} N exceeds capability of {max_n} N"
                )
            }
            Self::UnknownPathway(id) => write!(f, "unknown pathway {}", id.0),
            Self::NoPendingReinforcement => write!(f, "no reinforcement is pending"),
        }
    }
}

impl std::error::Error for ControlError {}

/// The single entry point for all manipulation logic.
///
/// Implement this for a new end-effector. A hardware driver, a simulator and
/// the mock `SimulatedHand` all use the same trait.
pub trait EndEffector: Send + Sync {
    /// Stable identity of this end-effector.
    fn identity(&self) -> &ToolIdentity;

    /// What the end-effector can do right now.
    fn capabilities(&self) -> &ToolCapabilities;

    /// Applies an intent. Returns an error if the intent is invalid or the
    /// end-effector cannot carry it out.
    fn command(&mut self, intent: ToolIntent) -> Result<(), ControlError>;

    /// Returns the next queued telemetry sample, or `None` if there is none.
    fn next_telemetry(&mut self) -> Option<ToolTelemetry>;

    /// Current physiological state, for tpt-soma sync.
    fn physiological_state(&self) -> PhysiologicalState;
}
