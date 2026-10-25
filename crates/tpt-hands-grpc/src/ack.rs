// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! The Ack method used by the interop tests. A `MorphogenesisRequest` names a
//! tool and a request id, and the reply echoes the id and approves it.

use tpt_hands_proto::generated::v1::{MorphogenesisAck, MorphogenesisRequest};

use crate::{Code, Server, Status};

/// Full method path for Ack.
pub const ACKNOWLEDGE_PATH: &str = "/tpt.hands.v1.Ack/Acknowledge";

/// Approves any request that names a tool.
pub async fn acknowledge(req: MorphogenesisRequest) -> Result<MorphogenesisAck, Status> {
    if req.tool_id.is_empty() {
        return Err(Status::new(Code::InvalidArgument, "tool_id is required"));
    }
    Ok(MorphogenesisAck {
        request_id: req.request_id,
        approved: true,
        ..Default::default()
    })
}

/// A server with the Ack method registered.
pub fn ack_server() -> Server {
    let mut server = Server::new();
    server.unary(ACKNOWLEDGE_PATH, acknowledge);
    server
}
