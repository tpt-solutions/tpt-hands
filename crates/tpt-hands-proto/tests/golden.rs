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
