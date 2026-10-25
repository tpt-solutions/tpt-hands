// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Manual interop check against soma's `interop_server` example, which is a tonic
//! server that decodes `DoPut` and replies with a row count. It does no
//! authentication and writes nothing, so this checks the wire format only.
//!
//! Start the server in tpt-soma, then run this test:
//!   cargo run -p tpt-soma-flight --example interop_server -- 127.0.0.1:50999
//!   SOMA_INTEROP_ADDR=127.0.0.1:50999 cargo test -p tpt-hands-soma-adapter --test interop -- --ignored

use std::time::Duration;

use tpt_hands_sdk::{PathwayHealth, PathwayId, PathwayState, PhysiologicalState};
use tpt_hands_soma_adapter::{BearerToken, SomaClient, batches};

#[tokio::test]
#[ignore = "needs soma's interop_server running at SOMA_INTEROP_ADDR"]
async fn interop_server_counts_the_rows_we_send() {
    let addr = std::env::var("SOMA_INTEROP_ADDR").unwrap_or_else(|_| "127.0.0.1:50999".into());
    let state = PhysiologicalState {
        pathways: vec![
            PathwayHealth {
                id: PathwayId(0),
                state: PathwayState::Intact,
                resistance_ohms: 10.0,
                max_bandwidth_mbps: 100.0,
            },
            PathwayHealth {
                id: PathwayId(1),
                state: PathwayState::Severed {
                    healing_progress: 0.5,
                },
                resistance_ohms: 480.0,
                max_bandwidth_mbps: 100.0,
            },
        ],
        structural_integrity: 0.75,
        estimated_recovery_time_ms: 1000,
        active_healing_protocols: Vec::new(),
    };
    let b = batches("hand-0", 42, &state).unwrap();
    let client = SomaClient::new(&addr, BearerToken::new("interop").unwrap());
    let rows = client.put(&b, Some(Duration::from_secs(10))).await.unwrap();
    assert_eq!(rows, 2);
}
