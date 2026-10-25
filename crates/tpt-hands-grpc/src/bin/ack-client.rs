// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Calls Ack on the address given as the first argument. Exits 0 only when the
//! reply is approved and echoes the request id.

use tpt_hands_grpc::Client;
use tpt_hands_grpc::ack::ACKNOWLEDGE_PATH;
use tpt_hands_proto::generated::v1::{MorphogenesisAck, MorphogenesisRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::args().nth(1).ok_or("usage: ack-client ADDR")?;
    let client = Client::new(&addr);

    let req = MorphogenesisRequest {
        tool_id: "hand-0".to_string(),
        request_id: "interop-1".to_string(),
        ..Default::default()
    };
    let ack: MorphogenesisAck = client.unary(ACKNOWLEDGE_PATH, &req, None).await?;
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
