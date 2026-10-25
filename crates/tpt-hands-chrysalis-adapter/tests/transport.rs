// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Tests the HTTP client against a MOCK of the proposed device-command interface.
//! The mock is not tpt-chrysalis. The real service does not have this endpoint
//! yet (see docs/ECOSYSTEM_INTEGRATION.md). The mock answers by command kind:
//!
//! - `accept`: 200 with a job id
//! - `refuse`: 422 with a gate reason
//! - `invalid`: 400
//! - `boom`: 500
//!
//! It also checks the bearer token and counts requests.

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bytes::Bytes;
use http::{Method, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tpt_hands_chrysalis_adapter::transport::{ChrysalisClient, submit_request};
use tpt_hands_chrysalis_adapter::{DEVICE_COMMAND_PATH, DeviceCommandBody, Outcome};
use tpt_hands_sdk::{MorphAction, MorphogenesisRequest, PathwayId, SafetyEnvelope, ToolIdentity};

const TOKEN: &str = "test-token";

struct Mock {
    base: String,
    requests: Arc<AtomicUsize>,
}

async fn start_mock() -> Mock {
    let lis = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", lis.local_addr().unwrap());
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = requests.clone();
    tokio::spawn(async move {
        loop {
            let (stream, _) = lis.accept().await.unwrap();
            let counter = counter.clone();
            tokio::spawn(async move {
                let svc = service_fn(move |req| {
                    let counter = counter.clone();
                    async move { Ok::<_, Infallible>(answer(req, counter).await) }
                });
                let _ = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), svc)
                    .await;
            });
        }
    });
    Mock { base, requests }
}

fn reply(status: StatusCode, body: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

async fn answer(req: hyper::Request<Incoming>, counter: Arc<AtomicUsize>) -> Response<Full<Bytes>> {
    counter.fetch_add(1, Ordering::SeqCst);
    if req.method() != Method::POST || req.uri().path() != DEVICE_COMMAND_PATH {
        return reply(StatusCode::NOT_FOUND, r#"{"error":"no such route"}"#);
    }
    let authorised = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == format!("Bearer {TOKEN}"));
    if !authorised {
        return reply(StatusCode::UNAUTHORIZED, r#"{"error":"bad token"}"#);
    }
    let bytes = req.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_default();
    let id = body["id"].as_str().unwrap_or_default().to_string();
    match body["kind"].as_str().unwrap_or_default() {
        "accept" => reply(StatusCode::OK, &format!(r#"{{"job_id":"job-{id}"}}"#)),
        "refuse" => reply(
            StatusCode::UNPROCESSABLE_ENTITY,
            r#"{"error":"gate: pressure over limit"}"#,
        ),
        "invalid" => reply(StatusCode::BAD_REQUEST, r#"{"error":"unknown kind"}"#),
        _ => reply(StatusCode::INTERNAL_SERVER_ERROR, "boom"),
    }
}

fn command(kind: &str, id: &str) -> DeviceCommandBody {
    DeviceCommandBody {
        id: id.to_string(),
        device: "hand-0".to_string(),
        kind: kind.to_string(),
        parameters: BTreeMap::new(),
        environment: BTreeMap::new(),
    }
}

#[tokio::test]
async fn accepted_command_yields_a_job_id() {
    let mock = start_mock().await;
    let client = ChrysalisClient::new(&mock.base, Some(TOKEN.to_string()));
    let outcome = client.submit(&command("accept", "c1")).await;
    assert_eq!(
        outcome,
        Outcome::Accepted {
            job_id: "job-c1".to_string()
        }
    );
    assert_eq!(mock.requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn refusal_carries_the_gate_reason() {
    let mock = start_mock().await;
    let client = ChrysalisClient::new(&mock.base, Some(TOKEN.to_string()));
    let outcome = client.submit(&command("refuse", "c2")).await;
    assert_eq!(
        outcome,
        Outcome::Refused {
            reason: "gate: pressure over limit".to_string()
        }
    );
}

#[tokio::test]
async fn bad_request_and_server_error_are_distinct() {
    let mock = start_mock().await;
    let client = ChrysalisClient::new(&mock.base, Some(TOKEN.to_string()));
    assert!(matches!(
        client.submit(&command("invalid", "c3")).await,
        Outcome::Invalid { .. }
    ));
    assert!(matches!(
        client.submit(&command("boom", "c4")).await,
        Outcome::Failed { .. }
    ));
}

#[tokio::test]
async fn wrong_token_is_a_failure_not_an_approval() {
    let mock = start_mock().await;
    let client = ChrysalisClient::new(&mock.base, Some("wrong".to_string()));
    let outcome = client.submit(&command("accept", "c5")).await;
    assert!(matches!(outcome, Outcome::Failed { .. }), "{outcome:?}");
}

#[tokio::test]
async fn missing_token_is_refused_before_any_request() {
    let mock = start_mock().await;
    let client = ChrysalisClient::new(&mock.base, None);
    let outcome = client.submit(&command("accept", "c6")).await;
    assert!(matches!(outcome, Outcome::Failed { .. }));
    assert_eq!(
        mock.requests.load(Ordering::SeqCst),
        0,
        "no request should leave"
    );
}

#[tokio::test]
async fn unreachable_service_is_a_failure() {
    // Port 9 on loopback is closed here, so the connection is refused.
    let client = ChrysalisClient::new("http://127.0.0.1:9", Some(TOKEN.to_string()));
    let outcome = client.submit(&command("accept", "c7")).await;
    assert!(matches!(outcome, Outcome::Failed { .. }), "{outcome:?}");
}

#[tokio::test]
async fn slow_service_times_out() {
    // A listener that accepts and never answers.
    let lis = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", lis.local_addr().unwrap());
    tokio::spawn(async move {
        let mut held = Vec::new();
        loop {
            if let Ok((s, _)) = lis.accept().await {
                held.push(s);
            }
        }
    });
    let client = ChrysalisClient::new(&base, Some(TOKEN.to_string()))
        .with_timeout(std::time::Duration::from_millis(200));
    let outcome = client.submit(&command("accept", "c8")).await;
    match outcome {
        Outcome::Failed { reason } => assert!(reason.contains("no answer"), "{reason}"),
        other => panic!("expected a timeout, got {other:?}"),
    }
}

fn request() -> MorphogenesisRequest {
    MorphogenesisRequest {
        target_endpoint: ToolIdentity::new("hand-0"),
        action: MorphAction::HealPathway {
            pathway_id: PathwayId(3),
            heat_assist: false,
        },
        safety_envelope: SafetyEnvelope {
            max_pressure_kpa: 200.0,
            max_temperature_c: 60.0,
            max_volume_ml: 5.0,
        },
    }
}

#[tokio::test]
async fn submit_request_maps_then_sends_then_acks() {
    let mock = start_mock().await;
    let client = ChrysalisClient::new(&mock.base, Some(TOKEN.to_string()));
    let ack = submit_request(&client, "req-9", &request()).await;
    // The mock answers by kind, and heal_pathway is not accept/refuse/invalid,
    // so the mock returns its generic failure.
    assert!(!ack.approved);
    assert_eq!(ack.request_id, "req-9");
    assert_eq!(mock.requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn unmappable_request_is_never_sent() {
    let mock = start_mock().await;
    let client = ChrysalisClient::new(&mock.base, Some(TOKEN.to_string()));
    let ack = submit_request(&client, "", &request()).await;
    assert!(!ack.approved);
    assert_eq!(mock.requests.load(Ordering::SeqCst), 0);
}
