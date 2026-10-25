// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use std::time::Duration;

use tokio::net::TcpListener;
use tpt_hands_grpc::ack::{ACKNOWLEDGE_PATH, ack_server};
use tpt_hands_grpc::{Client, Code, Server, Status};
use tpt_hands_proto::generated::v1::{MorphogenesisAck, MorphogenesisRequest};

/// Serves `server` on loopback and returns the address.
async fn start(server: Server) -> String {
    let lis = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = lis.local_addr().unwrap().to_string();
    tokio::spawn(async move {
        server.serve(lis).await.unwrap();
    });
    addr
}

fn request(tool: &str, id: &str) -> MorphogenesisRequest {
    MorphogenesisRequest {
        tool_id: tool.to_string(),
        request_id: id.to_string(),
        ..Default::default()
    }
}

#[tokio::test]
async fn unary_round_trip() {
    let client = Client::new(&start(ack_server()).await);
    let ack: MorphogenesisAck = client
        .unary(ACKNOWLEDGE_PATH, &request("hand-0", "r1"), None)
        .await
        .unwrap();
    assert!(ack.approved);
    assert_eq!(ack.request_id, "r1");
}

#[tokio::test]
async fn handler_status_reaches_the_client() {
    let client = Client::new(&start(ack_server()).await);
    let err = client
        .unary::<_, MorphogenesisAck>(ACKNOWLEDGE_PATH, &request("", "r2"), None)
        .await
        .unwrap_err();
    assert_eq!(err.code, Code::InvalidArgument);
    assert_eq!(err.message, "tool_id is required");
}

#[tokio::test]
async fn unknown_method_is_unimplemented() {
    let client = Client::new(&start(ack_server()).await);
    let err = client
        .unary::<_, MorphogenesisAck>("/tpt.hands.v1.Nope/Missing", &request("hand-0", "r3"), None)
        .await
        .unwrap_err();
    assert_eq!(err.code, Code::Unimplemented);
}

#[tokio::test]
async fn deadline_is_enforced_by_the_server() {
    let mut server = Server::new();
    server.unary("/test.Slow/Wait", |_: MorphogenesisRequest| async {
        tokio::time::sleep(Duration::from_secs(10)).await;
        Ok::<_, Status>(MorphogenesisAck::default())
    });
    let client = Client::new(&start(server).await);
    let started = std::time::Instant::now();
    let err = client
        .unary::<_, MorphogenesisAck>(
            "/test.Slow/Wait",
            &request("hand-0", "r4"),
            Some(Duration::from_millis(150)),
        )
        .await
        .unwrap_err();
    assert_eq!(err.code, Code::DeadlineExceeded);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn oversize_request_is_refused() {
    let client = Client::new(&start(ack_server().with_max_message_size(16)).await);
    let big = request("hand-0", &"x".repeat(200));
    let err = client
        .unary::<_, MorphogenesisAck>(ACKNOWLEDGE_PATH, &big, None)
        .await
        .unwrap_err();
    assert_eq!(err.code, Code::ResourceExhausted);
}

#[tokio::test]
async fn concurrent_calls_share_the_connection() {
    let client = std::sync::Arc::new(Client::new(&start(ack_server()).await));
    let mut tasks = Vec::new();
    for i in 0..32 {
        let client = client.clone();
        tasks.push(tokio::spawn(async move {
            let id = format!("c{i}");
            let ack: MorphogenesisAck = client
                .unary(ACKNOWLEDGE_PATH, &request("hand-0", &id), None)
                .await
                .unwrap();
            assert_eq!(ack.request_id, id);
        }));
    }
    for t in tasks {
        t.await.unwrap();
    }
}
