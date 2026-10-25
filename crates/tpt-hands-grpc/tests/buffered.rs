// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! `Client::call_buffered` against a raw hyper HTTP/2 server, which scripts the
//! exact response shapes the spec cares about: trailers-only errors, errors in the
//! trailers after zero or one message, truncated and oversize bodies, and deadlines.

use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
use tpt_hands_grpc::{Client, Code, encode_frame};

type Body = BoxBody<Bytes, Infallible>;
type Reply = Arc<dyn Fn() -> Response<Body> + Send + Sync>;

/// What the server saw on the last call.
#[derive(Default)]
struct Seen {
    calls: usize,
    headers: HeaderMap,
    frames: Vec<Vec<u8>>,
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

/// Starts a server that replies with `reply()` and records each request.
async fn serve(reply: Reply) -> (String, Arc<Mutex<Seen>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let seen = Arc::new(Mutex::new(Seen::default()));
    let seen_outer = seen.clone();
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let reply = reply.clone();
            let seen = seen_outer.clone();
            tokio::spawn(async move {
                let svc = service_fn(move |req: Request<Incoming>| {
                    let reply = reply.clone();
                    let seen = seen.clone();
                    async move {
                        let headers = req.headers().clone();
                        let body = req
                            .into_body()
                            .collect()
                            .await
                            .map(|c| c.to_bytes())
                            .unwrap_or_default();
                        {
                            let mut s = seen.lock().unwrap();
                            s.calls += 1;
                            s.headers = headers;
                            s.frames = parse_frames(&body);
                        }
                        Ok::<_, Infallible>(reply())
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

/// Data messages, then a trailer carrying `grpc-status` (and an optional message).
fn with_trailers(data: Vec<Bytes>, status: &str, message: Option<&str>) -> Response<Body> {
    let mut frames: Vec<Result<Frame<Bytes>, Infallible>> =
        data.into_iter().map(|d| Ok(Frame::data(d))).collect();
    let mut trailers = HeaderMap::new();
    trailers.insert("grpc-status", status.parse().unwrap());
    if let Some(m) = message {
        trailers.insert("grpc-message", m.parse().unwrap());
    }
    frames.push(Ok(Frame::trailers(trailers)));
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/grpc")
        .body(StreamBody::new(stream::iter(frames)).boxed())
        .unwrap()
}

/// A status that arrives in the headers, with no body: the trailers-only form.
fn trailers_only(status: &str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/grpc")
        .header("grpc-status", status)
        .body(Full::new(Bytes::new()).boxed())
        .unwrap()
}

/// Raw bytes as the data of an otherwise successful response.
fn raw_body(bytes: Vec<u8>) -> Response<Body> {
    with_trailers(vec![Bytes::from(bytes)], "0", None)
}

fn client(addr: &str) -> Client {
    Client::new(addr)
}

#[tokio::test]
async fn request_carries_metadata_frames_and_timeout() {
    let reply: Reply = Arc::new(|| with_trailers(vec![encode_frame(b"ack")], "0", None));
    let (addr, seen) = serve(reply).await;

    let got = client(&addr)
        .call_buffered(
            "/tpt.test.v1.Svc/Ingest",
            &[("authorization", "Bearer tok")],
            &[b"one".to_vec(), b"two".to_vec()],
            Some(Duration::from_secs(5)),
        )
        .await
        .unwrap();
    assert_eq!(got, vec![b"ack".to_vec()]);

    let s = seen.lock().unwrap();
    assert_eq!(s.calls, 1);
    assert_eq!(s.headers.get("authorization").unwrap(), "Bearer tok");
    assert_eq!(s.headers.get("te").unwrap(), "trailers");
    assert_eq!(s.headers.get(CONTENT_TYPE).unwrap(), "application/grpc");
    assert!(s.headers.get("grpc-timeout").is_some());
    assert_eq!(s.frames, vec![b"one".to_vec(), b"two".to_vec()]);
}

#[tokio::test]
async fn several_response_messages_come_back_in_order() {
    let reply: Reply = Arc::new(|| {
        with_trailers(
            vec![
                encode_frame(b"a"),
                encode_frame(b"bb"),
                encode_frame(b"ccc"),
            ],
            "0",
            None,
        )
    });
    let (addr, _) = serve(reply).await;
    let got = client(&addr)
        .call_buffered("/x/y", &[], &[], None)
        .await
        .unwrap();
    assert_eq!(got, vec![b"a".to_vec(), b"bb".to_vec(), b"ccc".to_vec()]);
}

#[tokio::test]
async fn trailers_only_error_is_returned() {
    let reply: Reply = Arc::new(|| trailers_only("7"));
    let (addr, _) = serve(reply).await;
    let err = client(&addr)
        .call_buffered("/x/y", &[], &[], None)
        .await
        .unwrap_err();
    assert_eq!(err.code, Code::PermissionDenied);
}

#[tokio::test]
async fn error_in_trailers_after_zero_messages_is_returned() {
    // The ingest path reports auth and validation failures like this.
    let reply: Reply = Arc::new(|| with_trailers(vec![], "16", Some("token expired")));
    let (addr, _) = serve(reply).await;
    let err = client(&addr)
        .call_buffered("/x/y", &[], &[], None)
        .await
        .unwrap_err();
    assert_eq!(err.code, Code::Unauthenticated);
    assert_eq!(err.message, "token expired");
}

#[tokio::test]
async fn error_after_one_message_discards_the_partial_result() {
    let reply: Reply =
        Arc::new(|| with_trailers(vec![encode_frame(b"partial")], "13", Some("write failed")));
    let (addr, _) = serve(reply).await;
    let err = client(&addr)
        .call_buffered("/x/y", &[], &[], None)
        .await
        .unwrap_err();
    assert_eq!(err.code, Code::Internal);
    assert_eq!(err.message, "write failed");
}

#[tokio::test]
async fn truncated_response_is_refused() {
    // Header claims 10 bytes, body carries 1.
    let reply: Reply = Arc::new(|| raw_body(vec![0, 0, 0, 0, 10, b'a']));
    let (addr, _) = serve(reply).await;
    let err = client(&addr)
        .call_buffered("/x/y", &[], &[], None)
        .await
        .unwrap_err();
    assert_eq!(err.code, Code::Internal);
}

#[tokio::test]
async fn oversize_response_is_refused() {
    let reply: Reply = Arc::new(|| with_trailers(vec![encode_frame(&[7u8; 100])], "0", None));
    let (addr, _) = serve(reply).await;
    let err = client(&addr)
        .with_max_message_size(16)
        .call_buffered("/x/y", &[], &[], None)
        .await
        .unwrap_err();
    assert_eq!(err.code, Code::ResourceExhausted);
}

#[tokio::test]
async fn deadline_is_enforced_on_the_client() {
    // A server that accepts the connection and never answers.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(5)).await;
    });
    let err = client(&addr)
        .call_buffered("/x/y", &[], &[], Some(Duration::from_millis(100)))
        .await
        .unwrap_err();
    assert_eq!(err.code, Code::DeadlineExceeded);
}

#[tokio::test]
async fn rejected_metadata_never_reaches_the_server() {
    let reply: Reply = Arc::new(|| with_trailers(vec![], "0", None));
    let (addr, seen) = serve(reply).await;
    for bad in [
        ("te", "x"),
        ("grpc-timeout", "1S"),
        ("x-data-bin", "AA=="),
        ("X-Upper", "1"),
    ] {
        let err = client(&addr)
            .call_buffered("/x/y", &[bad], &[], None)
            .await
            .unwrap_err();
        assert_eq!(err.code, Code::InvalidArgument, "{bad:?}");
    }
    assert_eq!(seen.lock().unwrap().calls, 0, "nothing may be sent");
}
