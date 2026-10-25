// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! MOCK of the proposed device-command interface. This is not tpt-chrysalis. The
//! real service has no `POST /api/v1/device-commands` endpoint yet (see
//! docs/ECOSYSTEM_INTEGRATION.md). The mock lets the adapter and the integration
//! tests run end to end. Its answers come from a closure the test supplies.

use std::convert::Infallible;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use http::{Method, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use serde_json::Value;
use tokio::net::TcpListener;

use crate::DEVICE_COMMAND_PATH;

type Answer = Arc<dyn Fn(&Value) -> (u16, String) + Send + Sync>;

/// A running mock on loopback.
pub struct Mock {
    /// Base URL, such as `http://127.0.0.1:41234`.
    pub base: String,
    requests: Arc<AtomicUsize>,
    bodies: Arc<Mutex<Vec<Value>>>,
}

impl Mock {
    /// Number of requests received, including refused ones.
    pub fn requests(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }

    /// Bodies of the requests that passed the token check.
    pub fn bodies(&self) -> Vec<Value> {
        self.bodies.lock().unwrap().clone()
    }
}

/// Starts the mock. A request without `Authorization: Bearer <token>` gets 401
/// and is not passed to `answer`. Everything else is answered by `answer`, which
/// gets the parsed command body and returns a status code and a body text.
pub async fn start<F>(token: &str, answer: F) -> Mock
where
    F: Fn(&Value) -> (u16, String) + Send + Sync + 'static,
{
    let lis = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", lis.local_addr().unwrap());
    let state = Arc::new(State {
        token: token.to_string(),
        answer: Arc::new(answer) as Answer,
        requests: Arc::new(AtomicUsize::new(0)),
        bodies: Arc::new(Mutex::new(Vec::new())),
    });
    let mock = Mock {
        base,
        requests: state.requests.clone(),
        bodies: state.bodies.clone(),
    };
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = lis.accept().await else {
                return;
            };
            let state = state.clone();
            tokio::spawn(async move {
                let svc = service_fn(move |req| {
                    let state = state.clone();
                    async move { Ok::<_, Infallible>(handle(&state, req).await) }
                });
                let _ = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), svc)
                    .await;
            });
        }
    });
    mock
}

struct State {
    token: String,
    answer: Answer,
    requests: Arc<AtomicUsize>,
    bodies: Arc<Mutex<Vec<Value>>>,
}

fn reply(status: StatusCode, body: String) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

async fn handle(state: &State, req: hyper::Request<Incoming>) -> Response<Full<Bytes>> {
    state.requests.fetch_add(1, Ordering::SeqCst);
    if req.method() != Method::POST || req.uri().path() != DEVICE_COMMAND_PATH {
        return reply(
            StatusCode::NOT_FOUND,
            r#"{"error":"no such route"}"#.to_string(),
        );
    }
    let authorised = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == format!("Bearer {}", state.token));
    if !authorised {
        return reply(
            StatusCode::UNAUTHORIZED,
            r#"{"error":"bad token"}"#.to_string(),
        );
    }
    let bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(_) => {
            return reply(
                StatusCode::BAD_REQUEST,
                r#"{"error":"unreadable body"}"#.to_string(),
            );
        }
    };
    let body: Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => {
            return reply(
                StatusCode::BAD_REQUEST,
                r#"{"error":"not JSON"}"#.to_string(),
            );
        }
    };
    state.bodies.lock().unwrap().push(body.clone());
    let (status, text) = (state.answer)(&body);
    reply(
        StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        text,
    )
}

/// The answers the adapter's own tests use, by command kind: `accept` gets a job
/// id, `refuse` gets the gate's 422, `invalid` gets 400, and anything else gets
/// a generic 500.
pub fn answer_by_kind(body: &Value) -> (u16, String) {
    let id = body["id"].as_str().unwrap_or_default();
    match body["kind"].as_str().unwrap_or_default() {
        "accept" => (200, format!(r#"{{"job_id":"job-{id}"}}"#)),
        "refuse" => (422, r#"{"error":"gate: pressure over limit"}"#.to_string()),
        "invalid" => (400, r#"{"error":"unknown kind"}"#.to_string()),
        _ => (500, "boom".to_string()),
    }
}
