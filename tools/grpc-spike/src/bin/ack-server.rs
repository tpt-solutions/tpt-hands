// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Serves the Ack spike on the address given as the first argument. Used by the
//! Go interop test in the orchestrator module.

use grpc_spike::{AckServer, ApproveAll};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::args().nth(1).ok_or("usage: ack-server ADDR")?;
    let lis = TcpListener::bind(&addr).await?;
    println!("listening {}", lis.local_addr()?);
    grpc_spike::serve(lis, AckServer::new(ApproveAll)).await?;
    Ok(())
}
