// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Golden wire fixtures. The hex strings were produced by Go's protobuf
//! implementation from `proto/tpt_hands.proto`, not by this codec. Each test
//! checks two things: this codec encodes the value to exactly those bytes, and
//! it decodes those bytes back to the value.

#![cfg(feature = "alloc")]

use tpt_hands_proto::wire::{
    FluidType, HealPathwayAction, MorphAction, MorphogenesisRequest, PathwayHealth, PathwayState,
    PhysiologicalUpdate, ReinforceTendonAction, SafetyEnvelope, SproutConduitAction,
};

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn physiological_update_matches_go() {
    let want = hex("0a0668616e642d30150000003f1a1110011d0000803e250000f0432d0000c8421a002087ad4b");
    let value = PhysiologicalUpdate {
        tool_id: "hand-0".into(),
        structural_integrity: 0.5,
        pathways: vec![
            PathwayHealth {
                id: 0,
                state: PathwayState::Severed,
                healing_progress: 0.25,
                resistance_ohms: 480.0,
                max_bandwidth_mbps: 100.0,
            },
            PathwayHealth::default(),
        ],
        timestamp_us: 1_234_567,
    };
    assert_eq!(to_hex(&value.encode()), to_hex(&want));
    assert_eq!(PhysiologicalUpdate::decode(&want).unwrap(), value);
}

#[test]
fn heal_request_matches_go() {
    let want =
        hex("0a0668616e642d3012057265712d315204080310019a060f0d0000484315000070421d0000a040");
    let value = MorphogenesisRequest {
        tool_id: "hand-0".into(),
        request_id: "req-1".into(),
        action: Some(MorphAction::Heal(HealPathwayAction {
            pathway_id: 3,
            heat_assist: true,
        })),
        safety_limits: Some(SafetyEnvelope {
            max_pressure_kpa: 200.0,
            max_temperature_c: 60.0,
            max_volume_ml: 5.0,
        }),
    };
    assert_eq!(to_hex(&value.encode()), to_hex(&want));
    assert_eq!(MorphogenesisRequest::decode(&want).unwrap(), value);
}

#[test]
fn sprout_request_matches_go() {
    let want = hex("0a0668616e642d3012057265712d325a06080110021802");
    let value = MorphogenesisRequest {
        tool_id: "hand-0".into(),
        request_id: "req-2".into(),
        action: Some(MorphAction::Sprout(SproutConduitAction {
            origin: 1,
            destination: 2,
            fluid_type: FluidType::LiquidMetal,
        })),
        safety_limits: None,
    };
    assert_eq!(to_hex(&value.encode()), to_hex(&want));
    assert_eq!(MorphogenesisRequest::decode(&want).unwrap(), value);
}

#[test]
fn reinforce_request_matches_go() {
    let want = hex("0a0668616e642d3012057265712d3362070804150000c03f");
    let value = MorphogenesisRequest {
        tool_id: "hand-0".into(),
        request_id: "req-3".into(),
        action: Some(MorphAction::Reinforce(ReinforceTendonAction {
            tendon_id: 4,
            volume_ml: 1.5,
        })),
        safety_limits: None,
    };
    assert_eq!(to_hex(&value.encode()), to_hex(&want));
    assert_eq!(MorphogenesisRequest::decode(&want).unwrap(), value);
}

#[test]
fn ack_matches_go() {
    use tpt_hands_proto::wire::MorphogenesisAck;
    let want = hex("0a057265712d3110011a066a6f622d3737");
    let value = MorphogenesisAck {
        request_id: "req-1".into(),
        approved: true,
        chrysalis_job_id: "job-77".into(),
    };
    assert_eq!(to_hex(&value.encode()), to_hex(&want));
    assert_eq!(MorphogenesisAck::decode(&want).unwrap(), value);
}

#[test]
fn zero_oneof_is_written_as_empty_field() {
    let want = hex("5200");
    let value = MorphogenesisRequest {
        action: Some(MorphAction::Heal(HealPathwayAction::default())),
        ..Default::default()
    };
    assert_eq!(to_hex(&value.encode()), to_hex(&want));
    assert_eq!(MorphogenesisRequest::decode(&want).unwrap(), value);
}

#[test]
fn default_message_is_empty_on_the_wire() {
    assert!(PhysiologicalUpdate::default().encode().is_empty());
    assert_eq!(
        PhysiologicalUpdate::decode(&[]).unwrap(),
        PhysiologicalUpdate::default()
    );
}
