// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Adapter that sends `MorphogenesisRequest`s to tpt-chrysalis.
//!
//! This crate turns a request into the body tpt-chrysalis accepts at
//! `POST /api/v1/device-commands`, reads the answer, and tracks the job through
//! its lifecycle. The mapping and the lifecycle do no I/O. The `http` feature (on
//! by default) adds the HTTP client in [`transport`], which is built against the
//! proposed interface, not the real service.
//!
//! Against the current tpt-chrysalis (commit `58a4dc6`), every command is refused
//! with 422 until its kind is approved. This adapter reports that refusal as an
//! ack with `approved = false`. Nothing is treated as done without an accepted
//! response.
//!
//! Safety limits are sent as parameters under their own names. They are not
//! converted to chrysalis gate variables, because the units differ (kPa against
//! `pressure_mbar`). That conversion is for the chrysalis owners to define.

#[cfg(feature = "http")]
pub mod transport;

/// MOCK of the proposed interface, for tests. Not the real tpt-chrysalis.
#[cfg(feature = "mock")]
pub mod mock;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tpt_hands_sdk::{FluidType, MorphAction, MorphogenesisRequest};

/// Path of the device-command endpoint on tpt-chrysalis.
pub const DEVICE_COMMAND_PATH: &str = "/api/v1/device-commands";

/// Body of `POST /api/v1/device-commands`. Field names match tpt-chrysalis
/// `DeviceCommandRequest`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DeviceCommandBody {
    pub id: String,
    pub device: String,
    pub kind: String,
    pub parameters: BTreeMap<String, f64>,
    pub environment: BTreeMap<String, f64>,
}

/// Why a request could not be turned into a command body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapError {
    /// The request id is empty.
    EmptyId,
    /// The target tool id is empty.
    EmptyToolId,
    /// A numeric field is NaN or infinite. Named by its parameter name.
    NonFinite(&'static str),
}

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyId => write!(f, "request id is empty"),
            Self::EmptyToolId => write!(f, "target tool id is empty"),
            Self::NonFinite(name) => write!(f, "`{name}` is not finite"),
        }
    }
}

impl std::error::Error for MapError {}

/// Wire code for a fluid. Chrysalis reads these as parameters.
fn fluid_code(fluid: FluidType) -> f64 {
    match fluid {
        FluidType::SelfHealingElastomer => 1.0,
        FluidType::LiquidMetal => 2.0,
        FluidType::Hydrogel => 3.0,
    }
}

fn finite(name: &'static str, value: f32) -> Result<f64, MapError> {
    if value.is_finite() {
        Ok(f64::from(value))
    } else {
        Err(MapError::NonFinite(name))
    }
}

/// Builds the device-command body for a morphogenesis request.
///
/// `id` is the request id, which the caller keeps to match the answer. The
/// target tool becomes `device`.
///
/// ```
/// use tpt_hands_chrysalis_adapter::to_device_command;
/// use tpt_hands_sdk::{MorphAction, MorphogenesisRequest, PathwayId, SafetyEnvelope, ToolIdentity};
///
/// let req = MorphogenesisRequest {
///     target_endpoint: ToolIdentity::new("hand-0"),
///     action: MorphAction::HealPathway { pathway_id: PathwayId(3), heat_assist: true },
///     safety_envelope: SafetyEnvelope {
///         max_pressure_kpa: 200.0,
///         max_temperature_c: 60.0,
///         max_volume_ml: 5.0,
///     },
/// };
/// let body = to_device_command("req-1", &req).unwrap();
/// assert_eq!(body.kind, "heal_pathway");
/// assert_eq!(body.parameters["pathway_id"], 3.0);
/// ```
pub fn to_device_command(
    id: &str,
    req: &MorphogenesisRequest,
) -> Result<DeviceCommandBody, MapError> {
    if id.is_empty() {
        return Err(MapError::EmptyId);
    }
    let device = req.target_endpoint.tool_id.as_str();
    if device.is_empty() {
        return Err(MapError::EmptyToolId);
    }

    let mut parameters = BTreeMap::new();
    let kind = match &req.action {
        MorphAction::HealPathway {
            pathway_id,
            heat_assist,
        } => {
            parameters.insert("pathway_id".to_string(), f64::from(pathway_id.0));
            parameters.insert(
                "heat_assist".to_string(),
                if *heat_assist { 1.0 } else { 0.0 },
            );
            "heal_pathway"
        }
        MorphAction::SproutConduit {
            origin,
            destination,
            fluid_type,
        } => {
            parameters.insert("origin".to_string(), f64::from(origin.0));
            parameters.insert("destination".to_string(), f64::from(destination.0));
            parameters.insert("fluid_type".to_string(), fluid_code(*fluid_type));
            "sprout_conduit"
        }
        MorphAction::ReinforceTendon {
            tendon_id,
            volume_ml,
        } => {
            parameters.insert("tendon_id".to_string(), f64::from(tendon_id.0));
            parameters.insert("volume_ml".to_string(), finite("volume_ml", *volume_ml)?);
            "reinforce_tendon"
        }
    };

    let envelope = &req.safety_envelope;
    parameters.insert(
        "max_pressure_kpa".to_string(),
        finite("max_pressure_kpa", envelope.max_pressure_kpa)?,
    );
    parameters.insert(
        "max_temperature_c".to_string(),
        finite("max_temperature_c", envelope.max_temperature_c)?,
    );
    parameters.insert(
        "max_volume_ml".to_string(),
        finite("max_volume_ml", envelope.max_volume_ml)?,
    );

    Ok(DeviceCommandBody {
        id: id.to_string(),
        device: device.to_string(),
        kind: kind.to_string(),
        parameters,
        environment: BTreeMap::new(),
    })
}

/// What chrysalis answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Accepted, with the job id chrysalis assigned.
    Accepted { job_id: String },
    /// Refused by the safety gate or by policy (HTTP 422). Carries the reason.
    Refused { reason: String },
    /// The request was malformed (HTTP 400).
    Invalid { reason: String },
    /// Anything else: no token, a server error, or an unexpected status.
    Failed { reason: String },
}

#[derive(Deserialize)]
struct ErrorBody {
    error: String,
}

fn reason_from(body: &str) -> String {
    match serde_json::from_str::<ErrorBody>(body) {
        Ok(e) => e.error,
        Err(_) => body.trim().to_string(),
    }
}

/// Reads an HTTP status and body from chrysalis into an [`Outcome`].
///
/// A 2xx answer is `Accepted`, and its body must have a `job_id` field. Without
/// one it is `Failed`, because a job that cannot be identified cannot be
/// tracked.
pub fn interpret(status: u16, body: &str) -> Outcome {
    match status {
        200..=299 => {
            #[derive(Deserialize)]
            struct Accepted {
                job_id: Option<String>,
            }
            match serde_json::from_str::<Accepted>(body)
                .ok()
                .and_then(|a| a.job_id)
            {
                Some(job_id) if !job_id.is_empty() => Outcome::Accepted { job_id },
                _ => Outcome::Failed {
                    reason: "accepted without a job id".to_string(),
                },
            }
        }
        400 => Outcome::Invalid {
            reason: reason_from(body),
        },
        422 => Outcome::Refused {
            reason: reason_from(body),
        },
        other => Outcome::Failed {
            reason: format!("HTTP {other}: {}", reason_from(body)),
        },
    }
}

/// The reply tpt-hands sends back for a request. Matches the proto
/// `MorphogenesisAck` (with `reason` added for refusals).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ack {
    pub request_id: String,
    pub approved: bool,
    pub chrysalis_job_id: String,
    pub reason: String,
}

/// Turns an outcome into the ack for a request.
pub fn ack_for(request_id: &str, outcome: &Outcome) -> Ack {
    match outcome {
        Outcome::Accepted { job_id } => Ack {
            request_id: request_id.to_string(),
            approved: true,
            chrysalis_job_id: job_id.clone(),
            reason: String::new(),
        },
        Outcome::Refused { reason } | Outcome::Invalid { reason } | Outcome::Failed { reason } => {
            Ack {
                request_id: request_id.to_string(),
                approved: false,
                chrysalis_job_id: String::new(),
                reason: reason.clone(),
            }
        }
    }
}

/// Where a job is in its life, as tpt-hands sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Submitted,
    Approved,
    Running,
    Done,
    Failed,
}

/// Something that happened to a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobEvent {
    Approved,
    Started,
    Completed,
    Failed,
}

/// An event that is not allowed from the current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionError {
    pub from: JobState,
    pub event: JobEvent,
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} cannot take {:?}", self.from, self.event)
    }
}

impl std::error::Error for TransitionError {}

/// Moves a job to its next state. `Done` and `Failed` are terminal.
///
/// ```
/// use tpt_hands_chrysalis_adapter::{advance, JobEvent, JobState};
///
/// let s = advance(JobState::Submitted, JobEvent::Approved).unwrap();
/// assert_eq!(s, JobState::Approved);
/// assert!(advance(JobState::Done, JobEvent::Started).is_err());
/// ```
pub fn advance(from: JobState, event: JobEvent) -> Result<JobState, TransitionError> {
    match (from, event) {
        (JobState::Submitted, JobEvent::Approved) => Ok(JobState::Approved),
        (JobState::Submitted, JobEvent::Failed)
        | (JobState::Approved, JobEvent::Failed)
        | (JobState::Running, JobEvent::Failed) => Ok(JobState::Failed),
        (JobState::Approved, JobEvent::Started) => Ok(JobState::Running),
        (JobState::Running, JobEvent::Completed) => Ok(JobState::Done),
        _ => Err(TransitionError { from, event }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_hands_sdk::{ChannelId, PathwayId, SafetyEnvelope, TendonId, ToolIdentity};

    fn envelope() -> SafetyEnvelope {
        SafetyEnvelope {
            max_pressure_kpa: 200.0,
            max_temperature_c: 60.0,
            max_volume_ml: 5.0,
        }
    }

    fn request(action: MorphAction) -> MorphogenesisRequest {
        MorphogenesisRequest {
            target_endpoint: ToolIdentity::new("hand-0"),
            action,
            safety_envelope: envelope(),
        }
    }

    #[test]
    fn heal_maps_to_heal_pathway() {
        let body = to_device_command(
            "req-1",
            &request(MorphAction::HealPathway {
                pathway_id: PathwayId(3),
                heat_assist: true,
            }),
        )
        .unwrap();
        assert_eq!(body.id, "req-1");
        assert_eq!(body.device, "hand-0");
        assert_eq!(body.kind, "heal_pathway");
        assert_eq!(body.parameters["pathway_id"], 3.0);
        assert_eq!(body.parameters["heat_assist"], 1.0);
        assert_eq!(body.parameters["max_pressure_kpa"], 200.0);
        assert!(body.environment.is_empty());
    }

    #[test]
    fn sprout_maps_fluid_to_its_code() {
        let body = to_device_command(
            "req-2",
            &request(MorphAction::SproutConduit {
                origin: ChannelId(1),
                destination: ChannelId(2),
                fluid_type: FluidType::Hydrogel,
            }),
        )
        .unwrap();
        assert_eq!(body.kind, "sprout_conduit");
        assert_eq!(body.parameters["origin"], 1.0);
        assert_eq!(body.parameters["destination"], 2.0);
        assert_eq!(body.parameters["fluid_type"], 3.0);
    }

    #[test]
    fn reinforce_maps_volume() {
        let body = to_device_command(
            "req-3",
            &request(MorphAction::ReinforceTendon {
                tendon_id: TendonId(4),
                volume_ml: 1.5,
            }),
        )
        .unwrap();
        assert_eq!(body.kind, "reinforce_tendon");
        assert_eq!(body.parameters["tendon_id"], 4.0);
        assert_eq!(body.parameters["volume_ml"], 1.5);
    }

    #[test]
    fn body_serialises_with_chrysalis_field_names() {
        let body = to_device_command(
            "req-1",
            &request(MorphAction::HealPathway {
                pathway_id: PathwayId(0),
                heat_assist: false,
            }),
        )
        .unwrap();
        let json: serde_json::Value = serde_json::to_value(&body).unwrap();
        for key in ["id", "device", "kind", "parameters", "environment"] {
            assert!(json.get(key).is_some(), "missing {key}");
        }
    }

    #[test]
    fn empty_ids_are_refused() {
        let action = MorphAction::HealPathway {
            pathway_id: PathwayId(0),
            heat_assist: false,
        };
        assert_eq!(
            to_device_command("", &request(action.clone())),
            Err(MapError::EmptyId)
        );
        let mut req = request(action);
        req.target_endpoint = ToolIdentity::new("");
        assert_eq!(to_device_command("r", &req), Err(MapError::EmptyToolId));
    }

    #[test]
    fn non_finite_values_are_refused_by_name() {
        let req = MorphogenesisRequest {
            safety_envelope: SafetyEnvelope {
                max_temperature_c: f32::NAN,
                ..envelope()
            },
            ..request(MorphAction::HealPathway {
                pathway_id: PathwayId(0),
                heat_assist: false,
            })
        };
        assert_eq!(
            to_device_command("r", &req),
            Err(MapError::NonFinite("max_temperature_c"))
        );
    }

    #[test]
    fn refusal_body_gives_the_reason() {
        let out = interpret(
            422,
            r#"{"error":"no approved safety rule for `heal_pathway`; command not executed"}"#,
        );
        assert_eq!(
            out,
            Outcome::Refused {
                reason: "no approved safety rule for `heal_pathway`; command not executed".into()
            }
        );
    }

    #[test]
    fn accepted_needs_a_job_id() {
        assert_eq!(
            interpret(201, r#"{"job_id":"job-9"}"#),
            Outcome::Accepted {
                job_id: "job-9".into()
            }
        );
        assert!(matches!(interpret(201, "{}"), Outcome::Failed { .. }));
        assert!(matches!(
            interpret(201, r#"{"job_id":""}"#),
            Outcome::Failed { .. }
        ));
    }

    #[test]
    fn statuses_map_to_outcomes() {
        assert!(matches!(
            interpret(400, r#"{"error":"bad"}"#),
            Outcome::Invalid { .. }
        ));
        assert!(matches!(
            interpret(401, r#"{"error":"no"}"#),
            Outcome::Failed { .. }
        ));
        assert!(matches!(interpret(500, "boom"), Outcome::Failed { .. }));
    }

    #[test]
    fn refusal_becomes_an_unapproved_ack_with_reason() {
        let ack = ack_for(
            "req-1",
            &Outcome::Refused {
                reason: "gate".into(),
            },
        );
        assert_eq!(
            ack,
            Ack {
                request_id: "req-1".into(),
                approved: false,
                chrysalis_job_id: String::new(),
                reason: "gate".into(),
            }
        );
    }

    #[test]
    fn acceptance_becomes_an_approved_ack() {
        let ack = ack_for(
            "req-1",
            &Outcome::Accepted {
                job_id: "job-9".into(),
            },
        );
        assert!(ack.approved);
        assert_eq!(ack.chrysalis_job_id, "job-9");
        assert!(ack.reason.is_empty());
    }

    #[test]
    fn job_follows_the_happy_path() {
        let mut s = JobState::Submitted;
        for e in [JobEvent::Approved, JobEvent::Started, JobEvent::Completed] {
            s = advance(s, e).unwrap();
        }
        assert_eq!(s, JobState::Done);
    }

    #[test]
    fn job_can_fail_from_any_live_state() {
        assert_eq!(
            advance(JobState::Submitted, JobEvent::Failed),
            Ok(JobState::Failed)
        );
        assert_eq!(
            advance(JobState::Approved, JobEvent::Failed),
            Ok(JobState::Failed)
        );
        assert_eq!(
            advance(JobState::Running, JobEvent::Failed),
            Ok(JobState::Failed)
        );
    }

    #[test]
    fn terminal_states_and_skips_are_refused() {
        assert!(advance(JobState::Submitted, JobEvent::Started).is_err());
        assert!(advance(JobState::Done, JobEvent::Failed).is_err());
        assert!(advance(JobState::Failed, JobEvent::Approved).is_err());
        assert!(advance(JobState::Running, JobEvent::Approved).is_err());
    }
}
