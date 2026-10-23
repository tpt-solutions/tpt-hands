// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Golden wire fixtures. The hex strings were produced by Go's protobuf
//! implementation from `proto/tpt_hands.proto`, not by this crate. Each test
//! checks two things: the generated code encodes the value to exactly those
//! bytes, and it decodes those bytes back to the value.

#![cfg(feature = "alloc")]

use micropb::{MessageDecode, MessageEncode, PbEncoder};
use tpt_hands_proto::generated::v1::MorphogenesisRequest_::Action;
use tpt_hands_proto::generated::v1::{
    FluidType, HealPathwayAction, MorphogenesisAck, MorphogenesisRequest, PathwayHealth,
    PathwayState, PhysiologicalUpdate, ReinforceTendonAction, SafetyEnvelope, SproutConduitAction,
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

fn encode<M: MessageEncode>(m: &M) -> Vec<u8> {
    let mut enc = PbEncoder::new(Vec::new());
    m.encode(&mut enc).unwrap();
    enc.into_writer()
}

fn decode<M: MessageDecode + Default>(bytes: &[u8]) -> M {
    let mut m = M::default();
    m.decode_from_bytes(bytes).unwrap();
    m
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
    assert_eq!(to_hex(&encode(&value)), to_hex(&want));
    assert_eq!(decode::<PhysiologicalUpdate>(&want), value);
}

#[test]
fn heal_request_matches_go() {
    let want =
        hex("0a0668616e642d3012057265712d315204080310019a060f0d0000484315000070421d0000a040");
    let mut value = MorphogenesisRequest {
        tool_id: "hand-0".into(),
        request_id: "req-1".into(),
        action: Some(Action::Heal(HealPathwayAction {
            pathway_id: 3,
            heat_assist: true,
        })),
        ..Default::default()
    };
    value.set_safety_limits(SafetyEnvelope {
        max_pressure_kpa: 200.0,
        max_temperature_c: 60.0,
        max_volume_ml: 5.0,
    });
    // Field order differs here: micropb writes safety_limits (99) before the
    // oneof (10), and Go writes them in field-number order. Both are valid
    // protobuf with the same meaning, so this test checks meaning, not bytes.
    assert_eq!(decode::<MorphogenesisRequest>(&want), value);
    assert_eq!(decode::<MorphogenesisRequest>(&encode(&value)), value);
}

#[test]
fn sprout_request_matches_go() {
    let want = hex("0a0668616e642d3012057265712d325a06080110021802");
    let value = MorphogenesisRequest {
        tool_id: "hand-0".into(),
        request_id: "req-2".into(),
        action: Some(Action::Sprout(SproutConduitAction {
            origin: 1,
            destination: 2,
            fluid_type: FluidType::LiquidMetal,
        })),
        ..Default::default()
    };
    assert_eq!(to_hex(&encode(&value)), to_hex(&want));
    assert_eq!(decode::<MorphogenesisRequest>(&want), value);
}

#[test]
fn reinforce_request_matches_go() {
    let want = hex("0a0668616e642d3012057265712d3362070804150000c03f");
    let value = MorphogenesisRequest {
        tool_id: "hand-0".into(),
        request_id: "req-3".into(),
        action: Some(Action::Reinforce(ReinforceTendonAction {
            tendon_id: 4,
            volume_ml: 1.5,
        })),
        ..Default::default()
    };
    assert_eq!(to_hex(&encode(&value)), to_hex(&want));
    assert_eq!(decode::<MorphogenesisRequest>(&want), value);
}

#[test]
fn ack_matches_go() {
    let want = hex("0a057265712d3110011a066a6f622d3737");
    let value = MorphogenesisAck {
        request_id: "req-1".into(),
        approved: true,
        chrysalis_job_id: "job-77".into(),
    };
    assert_eq!(to_hex(&encode(&value)), to_hex(&want));
    assert_eq!(decode::<MorphogenesisAck>(&want), value);
}

#[test]
fn zero_oneof_is_written_as_empty_field() {
    let want = hex("5200");
    let value = MorphogenesisRequest {
        action: Some(Action::Heal(HealPathwayAction::default())),
        ..Default::default()
    };
    assert_eq!(to_hex(&encode(&value)), to_hex(&want));
    assert_eq!(decode::<MorphogenesisRequest>(&want), value);
}

#[test]
fn default_message_is_empty_on_the_wire() {
    assert!(encode(&PhysiologicalUpdate::default()).is_empty());
    assert_eq!(
        decode::<PhysiologicalUpdate>(&[]),
        PhysiologicalUpdate::default()
    );
}

#[test]
fn unknown_fields_are_skipped() {
    // Field 50, varint 7, then a real ack.
    let mut bytes = hex("900307");
    bytes.extend(hex("0a057265712d3110011a066a6f622d3737"));
    let ack: MorphogenesisAck = decode(&bytes);
    assert_eq!(ack.request_id, "req-1");
    assert!(ack.approved);
    assert_eq!(ack.chrysalis_job_id, "job-77");
}

#[test]
fn truncated_input_is_an_error() {
    // String field 1 claims 5 bytes and gives 1.
    let mut m = PhysiologicalUpdate::default();
    assert!(m.decode_from_bytes(&[0x0A, 0x05, b'a']).is_err());
}

// ---- Telemetry, commands and power handshake (provisional messages) --------

use tpt_hands_proto::generated::v1::ToolCommand_::Intent;
use tpt_hands_proto::generated::v1::{
    CommandAck, GraspIntent, HandshakeHello, HandshakeResponse, PrepareReinforcementIntent,
    RelayState, ReleaseIntent, ToolCommand, ToolTelemetry,
};

#[test]
fn telemetry_matches_go() {
    let want = hex("0a0668616e642d30102a1d00004841250000803f");
    let value = ToolTelemetry {
        tool_id: "hand-0".into(),
        timestamp_us: 42,
        force_n: 12.5,
        structural_integrity: 1.0,
    };
    assert_eq!(to_hex(&encode(&value)), to_hex(&want));
    assert_eq!(decode::<ToolTelemetry>(&want), value);
}

#[test]
fn grasp_command_matches_go() {
    let want = hex("0a0668616e642d301205636d642d3952050d00004841");
    let value = ToolCommand {
        tool_id: "hand-0".into(),
        command_id: "cmd-9".into(),
        intent: Some(Intent::Grasp(GraspIntent { force_n: 12.5 })),
    };
    assert_eq!(to_hex(&encode(&value)), to_hex(&want));
    assert_eq!(decode::<ToolCommand>(&want), value);
}

#[test]
fn release_command_matches_go() {
    let want = hex("0a0668616e642d301206636d642d31305a00");
    let value = ToolCommand {
        tool_id: "hand-0".into(),
        command_id: "cmd-10".into(),
        intent: Some(Intent::Release(ReleaseIntent {})),
    };
    assert_eq!(to_hex(&encode(&value)), to_hex(&want));
    assert_eq!(decode::<ToolCommand>(&want), value);
}

#[test]
fn prepare_command_matches_go() {
    let want = hex("0a0668616e642d301206636d642d313162050d00004842");
    let value = ToolCommand {
        tool_id: "hand-0".into(),
        command_id: "cmd-11".into(),
        intent: Some(Intent::PrepareReinforcement(PrepareReinforcementIntent {
            target_force_n: 50.0,
        })),
    };
    assert_eq!(to_hex(&encode(&value)), to_hex(&want));
    assert_eq!(decode::<ToolCommand>(&want), value);
}

#[test]
fn rejected_command_ack_matches_go() {
    let want = hex("0a05636d642d391a1265786365656473206361706162696c697479");
    let value = CommandAck {
        command_id: "cmd-9".into(),
        accepted: false,
        reason: "exceeds capability".into(),
    };
    assert_eq!(to_hex(&encode(&value)), to_hex(&want));
    assert_eq!(decode::<CommandAck>(&want), value);
}

#[test]
fn handshake_hello_matches_go() {
    let want = hex("0a0668616e642d3012100102030405060708090a0b0c0d0e0f101807");
    let value = HandshakeHello {
        tool_id: "hand-0".into(),
        nonce: (1..=16).collect(),
        counter: 7,
    };
    assert_eq!(to_hex(&encode(&value)), to_hex(&want));
    assert_eq!(decode::<HandshakeHello>(&want), value);
}

#[test]
fn handshake_response_matches_go() {
    // 32-byte public key of 0xaa, then a 64-byte signature of 0xbb.
    let want = hex(&format!(
        "0a20{}1240{}1807",
        "aa".repeat(32),
        "bb".repeat(64)
    ));
    let value = HandshakeResponse {
        public_key: vec![0xaa; 32],
        signature: vec![0xbb; 64],
        counter: 7,
    };
    assert_eq!(to_hex(&encode(&value)), to_hex(&want));
    assert_eq!(decode::<HandshakeResponse>(&want), value);
}

#[test]
fn relay_state_matches_go() {
    let want = hex("0801");
    let value = RelayState {
        relay_closed: true,
        reason: String::new(),
    };
    assert_eq!(to_hex(&encode(&value)), to_hex(&want));
    assert_eq!(decode::<RelayState>(&want), value);
}
