// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Scenario tests for the tear, reroute, repair and heal loop. The chrysalis side
//! is a MOCK of the proposed interface (see docs/ECOSYSTEM_INTEGRATION.md).

use serde_json::Value;
use tpt_hands_chrysalis_adapter::mock::start;
use tpt_hands_chrysalis_adapter::transport::{ChrysalisClient, submit_request};
use tpt_hands_chrysalis_adapter::{JobEvent, JobState, advance};
use tpt_hands_edge::reflex::PathwayEvent;
use tpt_hands_integration::{Rig, repair_request};
use tpt_hands_sdk::PathwayId;

const TOKEN: &str = "integration-token";

/// Approves heal_pathway, as a chrysalis that accepts the repair would.
fn approve_heal(body: &Value) -> (u16, String) {
    match body["kind"].as_str() {
        Some("heal_pathway") => (
            200,
            format!(
                r#"{{"job_id":"job-{}"}}"#,
                body["id"].as_str().unwrap_or_default()
            ),
        ),
        _ => (500, "boom".to_string()),
    }
}

/// Refuses every command, as a gate with a pressure limit might.
fn refuse_all(_: &Value) -> (u16, String) {
    (422, r#"{"error":"gate: pressure over limit"}"#.to_string())
}

/// Tears pathway 0 and runs the edge until it reports the tear.
fn tear_and_detect(rig: &mut Rig) {
    rig.sample();
    rig.hand.tear(PathwayId(0)).unwrap();
    assert!(rig.step(10).is_empty(), "one sample must not change state");
    assert_eq!(rig.step(10), vec![(0, PathwayEvent::Severed)]);
}

/// Runs the hand for `total_ms` in `step_ms` steps and returns what the edge saw.
fn run(rig: &mut Rig, step_ms: u64, total_ms: u64) -> Vec<(usize, PathwayEvent)> {
    let mut seen = Vec::new();
    for _ in 0..(total_ms / step_ms) {
        seen.extend(rig.step(step_ms));
    }
    seen
}

#[tokio::test]
async fn the_edge_reroutes_a_tear_before_any_host_call() {
    let mut rig = Rig::new();
    assert_eq!(rig.route(), Some(0));
    tear_and_detect(&mut rig);
    assert_eq!(
        rig.route(),
        Some(1),
        "telemetry must move off the torn pathway"
    );
}

#[tokio::test]
async fn approved_repair_is_tracked_and_the_hand_heals_on_its_own_timer() {
    let mock = start(TOKEN, approve_heal).await;
    let client = ChrysalisClient::new(&mock.base, Some(TOKEN.to_string()));
    let mut rig = Rig::new();
    tear_and_detect(&mut rig);

    let ack = submit_request(&client, "req-1", &repair_request(0)).await;
    assert!(ack.approved, "{ack:?}");
    assert_eq!(ack.chrysalis_job_id, "job-req-1");

    let sent = mock.bodies();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0]["kind"], "heal_pathway");
    assert_eq!(sent[0]["parameters"]["pathway_id"].as_f64(), Some(0.0));

    // The job moves through its lifecycle as chrysalis reports it.
    let s = advance(JobState::Submitted, JobEvent::Approved).unwrap();
    let s = advance(s, JobEvent::Started).unwrap();
    assert_eq!(advance(s, JobEvent::Completed).unwrap(), JobState::Done);

    // The model heals after 2 s. Approval does not change that timer, and the
    // test does not claim it does.
    assert!(!run(&mut rig, 100, 1_000).contains(&(0, PathwayEvent::Healed)));
    let healed = run(&mut rig, 100, 1_500);
    assert!(healed.contains(&(0, PathwayEvent::Healed)));
    assert_eq!(
        rig.route(),
        Some(0),
        "telemetry returns to the preferred pathway"
    );
}

#[tokio::test]
async fn refused_repair_is_reported_and_leaves_the_reroute_in_place() {
    let mock = start(TOKEN, refuse_all).await;
    let client = ChrysalisClient::new(&mock.base, Some(TOKEN.to_string()));
    let mut rig = Rig::new();
    tear_and_detect(&mut rig);

    let ack = submit_request(&client, "req-2", &repair_request(0)).await;
    assert!(!ack.approved);
    assert_eq!(ack.reason, "gate: pressure over limit");
    assert!(
        ack.chrysalis_job_id.is_empty(),
        "no job exists for a refused command"
    );

    // A refused job never starts, so it fails from its first state.
    assert_eq!(
        advance(JobState::Submitted, JobEvent::Failed).unwrap(),
        JobState::Failed
    );

    // The reroute holds while the pathway is still torn.
    assert_eq!(rig.route(), Some(1));
}

#[tokio::test]
async fn unreachable_chrysalis_is_a_failed_ack_and_the_edge_keeps_routing() {
    // Port 9 on loopback is closed, so the connection is refused.
    let client = ChrysalisClient::new("http://127.0.0.1:9", Some(TOKEN.to_string()));
    let mut rig = Rig::new();
    tear_and_detect(&mut rig);

    let ack = submit_request(&client, "req-3", &repair_request(0)).await;
    assert!(!ack.approved);
    assert!(ack.reason.contains("transport"), "{}", ack.reason);

    // The host is unreachable, and telemetry still moves, because the edge does
    // not need the host to reroute.
    assert_eq!(rig.route(), Some(1));
}

#[tokio::test]
async fn link_drop_mid_heal_leaves_the_edge_to_finish_the_recovery() {
    let mut rig = Rig::new();
    tear_and_detect(&mut rig);

    // The host link drops here. No host calls happen, and no ack ever comes back.
    // The edge still sees the pathway heal, and telemetry returns to pathway 0.
    let healed = run(&mut rig, 100, 2_500);
    assert!(healed.contains(&(0, PathwayEvent::Healed)));
    assert_eq!(rig.route(), Some(0));
}

#[tokio::test]
async fn the_model_takes_its_healing_time_and_no_sooner() {
    let mut rig = Rig::new();
    tear_and_detect(&mut rig);
    let early = run(&mut rig, 100, 1_900);
    assert!(
        !early.contains(&(0, PathwayEvent::Healed)),
        "healed before the model's 2 s"
    );
}
