// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Core data types shared by the SDK, the adapters and the mock driver.

/// Identifies a pathway (a data or power conduit) inside an end-effector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PathwayId(pub u16);

/// Identifies a microfluidic channel used by a `SproutConduit` request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChannelId(pub u16);

/// Identifies a hydrogel tendon that can be reinforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TendonId(pub u16);

/// Identifies a healing protocol that tpt-chrysalis is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HealingProtocolId(pub u32);

/// Identifies an end-effector on the wire and in tpt-soma (`tool_id`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolIdentity {
    /// Stable identifier for this end-effector.
    pub tool_id: String,
}

impl ToolIdentity {
    /// Creates an identity from any string-like id.
    ///
    /// ```
    /// use tpt_hands_sdk::ToolIdentity;
    ///
    /// let id = ToolIdentity::new("hand-0");
    /// assert_eq!(id.tool_id, "hand-0");
    /// ```
    pub fn new(tool_id: impl Into<String>) -> Self {
        Self {
            tool_id: tool_id.into(),
        }
    }
}

/// Health of one pathway, as the edge reports it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathwayState {
    /// Working normally.
    Intact,
    /// Torn. `healing_progress` runs from 0.0 to 1.0, and 1.0 means healed.
    Severed {
        /// Fraction of the healing cycle that has completed.
        healing_progress: f32,
    },
    /// Thickened by reinforcement. Carries more load than an intact pathway.
    Hypertrophied,
}

/// Health of a single pathway.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathwayHealth {
    /// Which pathway this describes.
    pub id: PathwayId,
    /// Current state of the pathway.
    pub state: PathwayState,
    /// Measured resistance, used by the edge to detect tears.
    pub resistance_ohms: f32,
    /// Maximum bandwidth the pathway can carry.
    pub max_bandwidth_mbps: f32,
}

/// Physiological state, mirrored to tpt-soma (the digital twin).
#[derive(Debug, Clone, PartialEq)]
pub struct PhysiologicalState {
    /// Health of every pathway on the end-effector.
    pub pathways: Vec<PathwayHealth>,
    /// Overall integrity, from 0.0 (failed) to 1.0 (fully intact).
    pub structural_integrity: f32,
    /// Estimated time until all severed pathways have healed.
    pub estimated_recovery_time_ms: u64,
    /// Healing protocols currently running.
    pub active_healing_protocols: Vec<HealingProtocolId>,
}

/// Fluid used by microfluidic actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluidType {
    /// Self-healing elastomer precursor.
    SelfHealingElastomer,
    /// Liquid metal for data or power conduits.
    LiquidMetal,
    /// Fast-curing hydrogel for tendons.
    Hydrogel,
}

/// Physical limits that a `MorphogenesisRequest` must stay within.
///
/// tpt-chrysalis checks these against its safety invariants before it
/// actuates anything.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SafetyEnvelope {
    /// Maximum pump pressure in kPa.
    pub max_pressure_kpa: f32,
    /// Maximum temperature in degrees Celsius, for heat-assisted healing.
    pub max_temperature_c: f32,
    /// Maximum fluid volume in millilitres.
    pub max_volume_ml: f32,
}

/// The morphogenesis action to perform.
#[derive(Debug, Clone, PartialEq)]
pub enum MorphAction {
    /// Heal a severed pathway.
    HealPathway {
        /// Pathway to heal.
        pathway_id: PathwayId,
        /// Whether to apply mild heat to speed up curing.
        heat_assist: bool,
    },
    /// Grow a new conduit between two channels.
    SproutConduit {
        /// Channel the conduit starts from.
        origin: ChannelId,
        /// Channel the conduit ends at.
        destination: ChannelId,
        /// Fluid used to form the conduit.
        fluid_type: FluidType,
    },
    /// Thicken a tendon by pumping in hydrogel.
    ReinforceTendon {
        /// Tendon to reinforce.
        tendon_id: TendonId,
        /// Volume of hydrogel to inject, in millilitres.
        volume_ml: f32,
    },
}

/// A request that tpt-hands sends to tpt-chrysalis. It never actuates pumps
/// directly (principle P3).
#[derive(Debug, Clone, PartialEq)]
pub struct MorphogenesisRequest {
    /// End-effector the action applies to.
    pub target_endpoint: ToolIdentity,
    /// What to do.
    pub action: MorphAction,
    /// Limits the action must respect.
    pub safety_envelope: SafetyEnvelope,
}

/// What an end-effector can do right now. This is updated after reinforcement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolCapabilities {
    /// Maximum grip force in newtons.
    pub max_force_n: f32,
    /// Number of degrees of freedom.
    pub degrees_of_freedom: u8,
}

impl ToolCapabilities {
    /// Creates a capability set.
    ///
    /// ```
    /// use tpt_hands_sdk::ToolCapabilities;
    ///
    /// let caps = ToolCapabilities::new(30.0, 5);
    /// assert_eq!(caps.max_force_n, 30.0);
    /// ```
    pub fn new(max_force_n: f32, degrees_of_freedom: u8) -> Self {
        Self {
            max_force_n,
            degrees_of_freedom,
        }
    }
}
