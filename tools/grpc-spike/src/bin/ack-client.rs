// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Calls the Ack method on the address given as the first argument. Exits 0 only
//! when the ack comes back approved with the request id echoed.

use grpc_spike::{AckClient, MorphogenesisRequest};
use tonic::transport::Endpoint;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::args().nth(1).ok_or("usage: ack-client ADDR")?;
    let channel = Endpoint::from_shared(format!("http://{addr}"))?
        .connect()
        .await?;
    let mut client = AckClient::new(channel);

    let req = MorphogenesisRequest {
        tool_id: "hand-0".to_string(),
        request_id: "interop-1".to_string(),
        ..Default::default()
    };
    let ack = client.acknowledge(req).await?;
    if !ack.approved || ack.request_id != "interop-1" {
        return Err(format!(
            "unexpected ack: approved={} id={}",
            ack.approved, ack.request_id
        )
        .into());
    }
    println!("approved {}", ack.request_id);
    Ok(())
}
