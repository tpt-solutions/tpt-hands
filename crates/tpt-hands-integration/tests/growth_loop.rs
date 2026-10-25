// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Growth loop: an intent to handle a heavier load leads to a reinforcement
//! request, then recalibration, then new capabilities. The chrysalis side is a
//! MOCK of the proposed interface.

use serde_json::Value;
use tpt_hands_chrysalis_adapter::mock::start;
use tpt_hands_chrysalis_adapter::transport::{ChrysalisClient, submit_request};
use tpt_hands_integration::Rig;
use tpt_hands_sdk::ControlError;
use tpt_hands_sdk::{
    EndEffector, MorphAction, MorphogenesisRequest, SafetyEnvelope, TendonId, ToolIdentity,
    ToolIntent,
};

const TOKEN: &str = "growth-token";
const BASE_FORCE_N: f32 = 30.0;
const TARGET_FORCE_N: f32 = 45.0;

fn approve_reinforce(body: &Value) -> (u16, String) {
    match body["kind"].as_str() {
        Some("reinforce_tendon") => (
            200,
            format!(
                r#"{{"job_id":"job-{}"}}"#,
                body["id"].as_str().unwrap_or_default()
            ),
        ),
        _ => (500, "boom".to_string()),
    }
}

fn reinforce_request() -> MorphogenesisRequest {
    MorphogenesisRequest {
        target_endpoint: ToolIdentity::new("hand-0"),
        action: MorphAction::ReinforceTendon {
            tendon_id: TendonId(0),
            volume_ml: 1.0,
        },
        safety_envelope: SafetyEnvelope {
            max_pressure_kpa: 200.0,
            max_temperature_c: 60.0,
            max_volume_ml: 5.0,
        },
    }
}

/// Advances the hand by `ms` in 50 ms steps, so the recalibration timer ticks.
fn run(rig: &mut Rig, ms: u64) {
    for _ in 0..(ms / 50) {
        rig.step(50);
    }
}

#[tokio::test]
async fn approved_reinforcement_raises_the_force_limit_after_recalibration() {
    let mock = start(TOKEN, approve_reinforce).await;
    let client = ChrysalisClient::new(&mock.base, Some(TOKEN.to_string()));
    let mut rig = Rig::new();
    assert_eq!(rig.hand.capabilities().max_force_n, BASE_FORCE_N);

    // The intent: the caller needs to reach a higher force.
    rig.hand
        .command(ToolIntent::PrepareForReinforcement {
            target_force_n: TARGET_FORCE_N,
        })
        .unwrap();

    // The caller then asks chrysalis to reinforce, and chrysalis approves.
    let ack = submit_request(&client, "grow-1", &reinforce_request()).await;
    assert!(ack.approved, "{ack:?}");
    assert_eq!(mock.bodies()[0]["kind"], "reinforce_tendon");

    // Applying the approved reinforcement starts recalibration. The old limit
    // holds until it finishes.
    rig.hand.complete_reinforcement().unwrap();
    run(&mut rig, 400);
    assert_eq!(rig.hand.capabilities().max_force_n, BASE_FORCE_N);

    run(&mut rig, 200);
    assert_eq!(rig.hand.capabilities().max_force_n, TARGET_FORCE_N);
}

#[tokio::test]
async fn refused_reinforcement_leaves_the_force_limit_alone() {
    let mock = start(TOKEN, |_| {
        (422, r#"{"error":"gate: volume over limit"}"#.to_string())
    })
    .await;
    let client = ChrysalisClient::new(&mock.base, Some(TOKEN.to_string()));
    let mut rig = Rig::new();

    rig.hand
        .command(ToolIntent::PrepareForReinforcement {
            target_force_n: TARGET_FORCE_N,
        })
        .unwrap();

    let ack = submit_request(&client, "grow-2", &reinforce_request()).await;
    assert!(!ack.approved);
    assert_eq!(ack.reason, "gate: volume over limit");

    // No approval, so the reinforcement is not applied and the limit never moves.
    run(&mut rig, 1_000);
    assert_eq!(rig.hand.capabilities().max_force_n, BASE_FORCE_N);
}

#[tokio::test]
async fn reinforcement_without_an_intent_is_refused_by_the_hand() {
    let mut rig = Rig::new();
    assert_eq!(
        rig.hand.complete_reinforcement(),
        Err(ControlError::NoPendingReinforcement)
    );
    assert_eq!(rig.hand.capabilities().max_force_n, BASE_FORCE_N);
}
