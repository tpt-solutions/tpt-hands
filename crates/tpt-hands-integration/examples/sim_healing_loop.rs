// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Prints one tear-reroute-repair-heal loop as a timeline. The chrysalis side is a
//! MOCK of the proposed interface.
//!
//! Run with `cargo run -p tpt-hands-integration --example sim_healing_loop`.

use serde_json::Value;
use tpt_hands_chrysalis_adapter::mock::start;
use tpt_hands_chrysalis_adapter::transport::{ChrysalisClient, submit_request};
use tpt_hands_edge::reflex::PathwayEvent;
use tpt_hands_integration::{Rig, repair_request};
use tpt_hands_sdk::PathwayId;

const TOKEN: &str = "example-token";

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

#[tokio::main]
async fn main() {
    let mock = start(TOKEN, approve_heal).await;
    let client = ChrysalisClient::new(&mock.base, Some(TOKEN.to_string()));
    let mut rig = Rig::new();

    let mut t_ms = 0u64;
    let report = |t_ms: u64, what: &str, rig: &Rig| {
        println!("{t_ms:>6} ms  {what:<36} route={:?}", rig.route());
    };

    rig.sample();
    report(t_ms, "intact", &rig);

    rig.hand.tear(PathwayId(0)).unwrap();
    println!("{:>6} ms  tear injected on pathway 0", t_ms);
    for _ in 0..2 {
        t_ms += 10;
        for (i, event) in rig.step(10) {
            let what = match event {
                PathwayEvent::Severed => format!("edge: pathway {i} severed"),
                PathwayEvent::Healed => format!("edge: pathway {i} healed"),
                PathwayEvent::None => continue,
            };
            report(t_ms, &what, &rig);
        }
    }

    let ack = submit_request(&client, "req-demo", &repair_request(0)).await;
    println!(
        "{t_ms:>6} ms  chrysalis ack: approved={} job={:?} reason={:?}",
        ack.approved, ack.chrysalis_job_id, ack.reason
    );

    while t_ms < 3_000 {
        t_ms += 100;
        for (i, event) in rig.step(100) {
            if event == PathwayEvent::Healed {
                report(t_ms, &format!("edge: pathway {i} healed"), &rig);
            }
        }
    }
    report(t_ms, "end of run", &rig);
}
