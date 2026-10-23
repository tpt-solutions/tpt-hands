// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Public Rust SDK for tpt-hands.
//!
//! The SDK has the `EndEffector` trait, the core physiological and morphogenesis
//! types, and the `SimulatedHand` mock driver. The API is synchronous, so the
//! SDK does not depend on an async runtime.

#![forbid(unsafe_code)]

pub mod end_effector;
pub mod sim;
pub mod types;

pub use end_effector::{ControlError, EndEffector, ToolIntent, ToolTelemetry};
pub use sim::{Clock, ManualClock, MonotonicClock, SimulatedHand};
pub use types::{
    ChannelId, FluidType, HealingProtocolId, MorphAction, MorphogenesisRequest, PathwayHealth,
    PathwayId, PathwayState, PhysiologicalState, SafetyEnvelope, TendonId, ToolCapabilities,
    ToolIdentity,
};
