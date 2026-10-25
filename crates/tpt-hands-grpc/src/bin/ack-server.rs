// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Serves Ack on the address given as the first argument. The Go interop test
//! in the orchestrator module runs this.

use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::args().nth(1).ok_or("usage: ack-server ADDR")?;
    let lis = TcpListener::bind(&addr).await?;
    println!("listening {}", lis.local_addr()?);
    tpt_hands_grpc::ack::ack_server().serve(lis).await?;
    Ok(())
}
