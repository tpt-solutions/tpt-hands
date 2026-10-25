// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! `SomaClient::put` against a mock soma that speaks gRPC over HTTP/2. It records
//! what the client sent and replies the way soma does: a `PutResult` with the row
//! count, or an error status in the trailers.

use std::convert::Infallible;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use futures_util::stream;
use http::header::CONTENT_TYPE;
use http::{HeaderMap, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full, StreamBody, combinators::BoxBody};
use hyper::body::{Frame, Incoming};
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use tokio::net::TcpListener;
use tpt_hands_sdk::{PathwayHealth, PathwayId, PathwayState, PhysiologicalState};
use tpt_hands_soma_adapter::{AdapterError, BearerToken, SomaClient, batches};
use tpt_soma_flight::data::flight_data_to_batches;
use tpt_soma_flight::pb::{FlightData, Message, PutResult};

type Body = BoxBody<Bytes, Infallible>;

#[derive(Default)]
struct Seen {
    calls: usize,
    path: String,
    authorization: Option<String>,
    messages: Vec<Vec<u8>>,
}

fn parse_frames(mut b: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    while b.len() >= 5 {
        let n = u32::from_be_bytes([b[1], b[2], b[3], b[4]]) as usize;
        out.push(b[5..5 + n].to_vec());
        b = &b[5 + n..];
    }
    out
}

/// Replies with a `PutResult` for `rows`, or with `error` as a trailer status.
async fn serve(
    rows: Option<u64>,
    error: Option<(&'static str, &'static str)>,
) -> (String, Arc<Mutex<Seen>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let seen = Arc::new(Mutex::new(Seen::default()));
    let seen_outer = seen.clone();
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let seen = seen_outer.clone();
            tokio::spawn(async move {
                let svc = service_fn(move |req: Request<Incoming>| {
                    let seen = seen.clone();
                    async move {
                        let path = req.uri().path().to_string();
                        let headers: HeaderMap = req.headers().clone();
                        let body = req
                            .into_body()
                            .collect()
                            .await
                            .map(|c| c.to_bytes())
                            .unwrap_or_default();
                        {
                            let mut s = seen.lock().unwrap();
                            s.calls += 1;
                            s.path = path;
                            s.authorization = headers
                                .get("authorization")
                                .and_then(|v| v.to_str().ok())
                                .map(str::to_string);
                            s.messages = parse_frames(&body);
                        }
                        Ok::<_, Infallible>(reply(rows, error))
                    }
                });
                let _ = Builder::new(TokioExecutor::new())
                    .http2_only()
                    .serve_connection(TokioIo::new(stream), svc)
                    .await;
            });
        }
    });
    (addr, seen)
}

fn reply(rows: Option<u64>, error: Option<(&'static str, &'static str)>) -> Response<Body> {
    if let Some((code, message)) = error {
        // Trailers-only: the status arrives in the headers, as for auth failures.
        return Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "application/grpc")
            .header("grpc-status", code)
            .header("grpc-message", message)
            .body(Full::new(Bytes::new()).boxed())
            .unwrap();
    }
    let meta = format!("{{\"rows\":{}}}", rows.unwrap_or(0));
    let payload = PutResult {
        app_metadata: Bytes::from(meta),
    }
    .encode_to_vec();
    let mut framed = vec![0u8];
    framed.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    framed.extend_from_slice(&payload);
    let mut trailers = HeaderMap::new();
    trailers.insert("grpc-status", "0".parse().unwrap());
    let frames: Vec<Result<Frame<Bytes>, Infallible>> = vec![
        Ok(Frame::data(Bytes::from(framed))),
        Ok(Frame::trailers(trailers)),
    ];
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/grpc")
        .body(StreamBody::new(stream::iter(frames)).boxed())
        .unwrap()
}

fn snapshot() -> PhysiologicalState {
    PhysiologicalState {
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
                    healing_progress: 0.25,
                },
                resistance_ohms: 480.0,
                max_bandwidth_mbps: 100.0,
            },
        ],
        structural_integrity: 0.6,
        estimated_recovery_time_ms: 1500,
        active_healing_protocols: Vec::new(),
    }
}

fn token() -> BearerToken {
    BearerToken::new("{\"subject\":\"hands-test\",\"action\":\"ingest\"}").unwrap()
}

#[tokio::test]
async fn put_sends_one_authorised_call_and_returns_the_row_count() {
    let (addr, seen) = serve(Some(2), None).await;
    let client = SomaClient::new(&addr, token());
    let b = batches("hand-0", 42, &snapshot()).unwrap();

    let rows = client.put(&b, None).await.unwrap();
    assert_eq!(rows, 2);

    let s = seen.lock().unwrap();
    assert_eq!(s.calls, 1);
    assert_eq!(s.path, "/arrow.flight.protocol.FlightService/DoPut");
    assert_eq!(
        s.authorization.as_deref(),
        Some("Bearer {\"subject\":\"hands-test\",\"action\":\"ingest\"}")
    );
    // One schema message, then one batch message.
    assert_eq!(s.messages.len(), 1 + b.len());

    // What soma receives decodes back to exactly what the adapter built.
    let decoded: Vec<FlightData> = s
        .messages
        .iter()
        .map(|m| FlightData::decode_raw(m).unwrap())
        .collect();
    assert_eq!(
        &decoded[0].flight_descriptor.as_ref().unwrap().cmd[..],
        b"device_state"
    );
    assert_eq!(flight_data_to_batches(&decoded).unwrap(), b);
}

#[tokio::test]
async fn unauthenticated_reply_is_reported_distinctly() {
    let (addr, _) = serve(None, Some(("16", "token expired"))).await;
    let client = SomaClient::new(&addr, token());
    let b = batches("hand-0", 42, &snapshot()).unwrap();
    match client.put(&b, None).await {
        Err(AdapterError::Unauthenticated(m)) => assert_eq!(m, "token expired"),
        other => panic!("expected Unauthenticated, got {other:?}"),
    }
}

#[tokio::test]
async fn permission_and_validation_errors_map_to_their_own_variants() {
    let b = batches("hand-0", 42, &snapshot()).unwrap();

    let (addr, _) = serve(None, Some(("7", "wrong class"))).await;
    assert!(matches!(
        SomaClient::new(&addr, token()).put(&b, None).await,
        Err(AdapterError::PermissionDenied(_))
    ));

    let (addr, _) = serve(None, Some(("3", "bad batch"))).await;
    assert!(matches!(
        SomaClient::new(&addr, token()).put(&b, None).await,
        Err(AdapterError::Rejected(_))
    ));
}

#[tokio::test]
async fn nothing_is_sent_when_the_snapshot_is_refused() {
    let (_addr, seen) = serve(Some(0), None).await;
    let bad = PhysiologicalState {
        structural_integrity: f32::NAN,
        ..snapshot()
    };
    // The refusal happens while building batches, before any call is made.
    assert!(matches!(
        batches("hand-0", 42, &bad),
        Err(AdapterError::NonFinite("structural_integrity"))
    ));
    assert_eq!(seen.lock().unwrap().calls, 0);
}
