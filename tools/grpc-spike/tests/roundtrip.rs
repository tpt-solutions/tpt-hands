// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use grpc_spike::{
    AckClient, AckServer, ApproveAll, MicropbCodec, MorphogenesisAck, MorphogenesisRequest,
};
use tokio::net::TcpListener;
use tonic::Code;
use tonic::transport::{Channel, Endpoint};

/// Starts the Ack service on loopback and returns the client channel.
async fn start() -> Channel {
    let lis = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = lis.local_addr().unwrap();
    tokio::spawn(async move {
        grpc_spike::serve(lis, AckServer::new(ApproveAll))
            .await
            .unwrap();
    });
    Endpoint::from_shared(format!("http://{addr}"))
        .unwrap()
        .connect()
        .await
        .unwrap()
}

fn request(tool: &str, id: &str) -> MorphogenesisRequest {
    MorphogenesisRequest {
        tool_id: tool.to_string(),
        request_id: id.to_string(),
        ..Default::default()
    }
}

#[tokio::test]
async fn unary_round_trip_through_micropb_codec() {
    let channel = start().await;
    let mut client = AckClient::new(channel);
    let ack: MorphogenesisAck = client
        .acknowledge(request("hand-0", "req-1"))
        .await
        .unwrap();
    assert!(ack.approved);
    assert_eq!(ack.request_id, "req-1");
}

#[tokio::test]
async fn application_error_comes_back_as_status() {
    let channel = start().await;
    let mut client = AckClient::new(channel);
    let err = client.acknowledge(request("", "req-2")).await.unwrap_err();
    assert_eq!(err.code(), Code::InvalidArgument);
}

#[tokio::test]
async fn unknown_method_is_unimplemented() {
    let channel = start().await;
    let mut grpc = tonic::client::Grpc::new(channel);
    grpc.ready().await.unwrap();
    let path = tonic::codegen::http::uri::PathAndQuery::from_static("/tpt.hands.v1.Nope/Missing");
    let codec: MicropbCodec<MorphogenesisRequest, MorphogenesisAck> = MicropbCodec::default();
    let err = grpc
        .unary(tonic::Request::new(request("hand-0", "x")), path, codec)
        .await
        .unwrap_err();
    assert_eq!(err.code(), Code::Unimplemented);
}
