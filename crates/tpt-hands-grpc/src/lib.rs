// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! gRPC over HTTP/2 for the Rust host, with micropb messages.
//!
//! This is the Rust counterpart of `go/tpt-grpc-lite`. It is written on hyper
//! and micropb, not tonic: tonic depends on `sync_wrapper`, which is Apache-2.0
//! only, and the licence policy bans that.
//!
//! Scope: unary calls, buffered multi-message calls with request metadata, status
//! trailers, deadlines (`grpc-timeout`), message size limits, and cleartext
//! HTTP/2. Not yet: true streaming, response metadata, TLS, compression, and
//! interceptors.

pub mod ack;

use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};
use futures_util::stream;
use http::header::{CONTENT_TYPE, HeaderName, HeaderValue};
use http::{HeaderMap, Method, Request, Response, StatusCode};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full, Limited, StreamBody};
use hyper::body::{Frame, Incoming};
use hyper::service::service_fn;
use hyper_util::client::legacy::Client as HyperClient;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as ServerBuilder;
use micropb::{MessageDecode, MessageEncode, PbEncoder};
use tokio::net::TcpListener;

/// Largest message accepted by default. Matches the gRPC default and the Go side.
pub const DEFAULT_MAX_MESSAGE: usize = 4 << 20;

const GRPC_CONTENT_TYPE: &str = "application/grpc";
const GRPC_STATUS: HeaderName = HeaderName::from_static("grpc-status");
const GRPC_MESSAGE: HeaderName = HeaderName::from_static("grpc-message");
const GRPC_TIMEOUT: HeaderName = HeaderName::from_static("grpc-timeout");
const FRAME_HEADER_LEN: usize = 5;

/// gRPC status codes, as in the gRPC specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    Ok,
    Cancelled,
    Unknown,
    InvalidArgument,
    DeadlineExceeded,
    NotFound,
    AlreadyExists,
    PermissionDenied,
    ResourceExhausted,
    FailedPrecondition,
    Aborted,
    OutOfRange,
    Unimplemented,
    Internal,
    Unavailable,
    DataLoss,
    Unauthenticated,
}

impl Code {
    pub fn as_u32(self) -> u32 {
        self as u32
    }

    fn from_u32(n: u32) -> Option<Code> {
        use Code::*;
        Some(match n {
            0 => Ok,
            1 => Cancelled,
            2 => Unknown,
            3 => InvalidArgument,
            4 => DeadlineExceeded,
            5 => NotFound,
            6 => AlreadyExists,
            7 => PermissionDenied,
            8 => ResourceExhausted,
            9 => FailedPrecondition,
            10 => Aborted,
            11 => OutOfRange,
            12 => Unimplemented,
            13 => Internal,
            14 => Unavailable,
            15 => DataLoss,
            16 => Unauthenticated,
            _ => return None,
        })
    }
}

/// A call's final status. A handler returns one to set what the client sees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub code: Code,
    pub message: String,
}

impl Status {
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "grpc status {:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for Status {}

/// Wraps one message in the 5-byte gRPC frame: a compressed flag (always 0) and a
/// big-endian length.
pub fn encode_frame(payload: &[u8]) -> Bytes {
    let mut out = BytesMut::with_capacity(FRAME_HEADER_LEN + payload.len());
    out.put_u8(0);
    out.put_u32(payload.len() as u32);
    out.put_slice(payload);
    out.freeze()
}

/// Reads the single message in a unary body. Anything other than exactly one
/// uncompressed frame is `InvalidArgument`, and an oversize message is
/// `ResourceExhausted`.
pub fn decode_frame(buf: &[u8], max: usize) -> Result<&[u8], Status> {
    if buf.len() < FRAME_HEADER_LEN {
        return Err(Status::new(Code::Internal, "truncated frame header"));
    }
    if buf[0] != 0 {
        return Err(Status::new(
            Code::Unimplemented,
            "compressed messages are not supported",
        ));
    }
    let n = u32::from_be_bytes([buf[1], buf[2], buf[3], buf[4]]) as usize;
    if n > max {
        return Err(Status::new(
            Code::ResourceExhausted,
            format!("message of {n} bytes exceeds limit of {max}"),
        ));
    }
    if buf.len() != FRAME_HEADER_LEN + n {
        return Err(Status::new(
            Code::Internal,
            "frame length does not match body",
        ));
    }
    Ok(&buf[FRAME_HEADER_LEN..])
}

fn encode_msg<M: MessageEncode>(m: &M) -> Result<Vec<u8>, Status> {
    let mut enc = PbEncoder::new(Vec::new());
    m.encode(&mut enc)
        .map_err(|e| Status::new(Code::Internal, format!("encode: {e:?}")))?;
    Ok(enc.into_writer())
}

fn decode_msg<M: MessageDecode + Default>(bytes: &[u8]) -> Result<M, Status> {
    let mut m = M::default();
    m.decode_from_bytes(bytes)
        .map_err(|e| Status::new(Code::InvalidArgument, format!("decode: {e:?}")))?;
    Ok(m)
}

/// Percent-encodes a status message, as the grpc-message trailer requires.
pub fn encode_message(msg: &str) -> String {
    let mut out = String::with_capacity(msg.len());
    for b in msg.bytes() {
        if (0x20..=0x7e).contains(&b) && b != b'%' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Reverses [`encode_message`]. A malformed escape is kept as is.
pub fn decode_message(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

const TIMEOUT_UNITS: [(char, u128); 6] = [
    ('H', 3_600_000_000_000),
    ('M', 60_000_000_000),
    ('S', 1_000_000_000),
    ('m', 1_000_000),
    ('u', 1_000),
    ('n', 1),
];

/// Formats a duration as a `grpc-timeout` value: at most 8 digits and one unit
/// letter. Rounds down, so a deadline is never extended.
pub fn encode_timeout(d: Duration) -> String {
    let nanos = d.as_nanos();
    for (letter, unit) in TIMEOUT_UNITS.iter().rev() {
        let v = nanos / unit;
        if v < 100_000_000 {
            return format!("{v}{letter}");
        }
    }
    "99999999H".to_string()
}

/// Parses a `grpc-timeout` value.
pub fn parse_timeout(s: &str) -> Option<Duration> {
    if s.len() < 2 || s.len() > 9 {
        return None;
    }
    let (digits, unit) = s.split_at(s.len() - 1);
    let n: u128 = digits.parse().ok()?;
    let letter = unit.chars().next()?;
    let (_, scale) = TIMEOUT_UNITS.iter().find(|(l, _)| *l == letter)?;
    let nanos = n.checked_mul(*scale)?;
    Some(Duration::from_nanos(u64::try_from(nanos).ok()?))
}

type HandlerFuture = Pin<Box<dyn Future<Output = Result<Vec<u8>, Status>> + Send>>;
type Handler = Arc<dyn Fn(Vec<u8>) -> HandlerFuture + Send + Sync>;

/// Serves unary methods over HTTP/2.
pub struct Server {
    methods: HashMap<String, Handler>,
    max_recv: usize,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn new() -> Self {
        Self {
            methods: HashMap::new(),
            max_recv: DEFAULT_MAX_MESSAGE,
        }
    }

    /// Sets the largest request message accepted.
    pub fn with_max_message_size(mut self, n: usize) -> Self {
        self.max_recv = n;
        self
    }

    /// Registers a unary method at `path`, such as `/pkg.Service/Method`.
    pub fn unary<Req, Resp, F, Fut>(&mut self, path: &str, f: F)
    where
        Req: MessageDecode + Default + Send + 'static,
        Resp: MessageEncode + Send + 'static,
        F: Fn(Req) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Resp, Status>> + Send + 'static,
    {
        let f = Arc::new(f);
        let handler: Handler = Arc::new(move |bytes: Vec<u8>| {
            let f = f.clone();
            Box::pin(async move {
                let req: Req = decode_msg(&bytes)?;
                let resp = f(req).await?;
                encode_msg(&resp)
            })
        });
        self.methods.insert(path.to_string(), handler);
    }

    /// Serves cleartext HTTP/2 on `lis` until the listener fails.
    pub async fn serve(self, lis: TcpListener) -> io::Result<()> {
        let shared = Arc::new(self);
        loop {
            let (stream, _peer) = lis.accept().await?;
            let shared = shared.clone();
            tokio::spawn(async move {
                let svc = service_fn(move |req| {
                    let shared = shared.clone();
                    async move { Ok::<_, Infallible>(shared.handle(req).await) }
                });
                // A failing connection only affects itself.
                let _ = ServerBuilder::new(TokioExecutor::new())
                    .http2_only()
                    .serve_connection(TokioIo::new(stream), svc)
                    .await;
            });
        }
    }

    async fn handle(&self, req: Request<Incoming>) -> Response<BoxBody<Bytes, Infallible>> {
        if req.method() != Method::POST {
            return plain(StatusCode::METHOD_NOT_ALLOWED);
        }
        let is_grpc = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|ct| ct.starts_with(GRPC_CONTENT_TYPE));
        if !is_grpc {
            return plain(StatusCode::UNSUPPORTED_MEDIA_TYPE);
        }

        let timeout = match req.headers().get(&GRPC_TIMEOUT) {
            None => None,
            Some(v) => match v.to_str().ok().and_then(parse_timeout) {
                Some(d) => Some(d),
                None => {
                    return trailers_only(Status::new(Code::InvalidArgument, "bad grpc-timeout"));
                }
            },
        };

        let path = req.uri().path().to_string();
        let Some(handler) = self.methods.get(&path).cloned() else {
            return trailers_only(Status::new(
                Code::Unimplemented,
                format!("unknown method {path}"),
            ));
        };

        // Limit the read so an oversize body is refused without buffering it.
        let body = match Limited::new(req.into_body(), self.max_recv + FRAME_HEADER_LEN)
            .collect()
            .await
        {
            Ok(c) => c.to_bytes(),
            Err(e) => {
                let status = if e
                    .downcast_ref::<http_body_util::LengthLimitError>()
                    .is_some()
                {
                    Status::new(Code::ResourceExhausted, "request message too large")
                } else {
                    Status::new(Code::Unknown, "failed to read request body")
                };
                return trailers_only(status);
            }
        };

        let payload = match decode_frame(&body, self.max_recv) {
            Ok(p) => p.to_vec(),
            Err(status) => return trailers_only(status),
        };

        let call = handler(payload);
        let result = match timeout {
            Some(d) => tokio::time::timeout(d, call)
                .await
                .unwrap_or_else(|_| Err(Status::new(Code::DeadlineExceeded, "deadline exceeded"))),
            None => call.await,
        };
        match result {
            Ok(bytes) => ok_response(bytes),
            Err(status) => trailers_only(status),
        }
    }
}

/// Response for a successful call: one framed message, then the OK status in the
/// trailers.
fn ok_response(payload: Vec<u8>) -> Response<BoxBody<Bytes, Infallible>> {
    let mut trailers = HeaderMap::new();
    trailers.insert(GRPC_STATUS, HeaderValue::from_static("0"));
    let frames = vec![
        Ok::<_, Infallible>(Frame::data(encode_frame(&payload))),
        Ok(Frame::trailers(trailers)),
    ];
    let body = StreamBody::new(stream::iter(frames)).boxed();
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, GRPC_CONTENT_TYPE)
        .body(body)
        .expect("static response is valid")
}

/// Response that carries only the status, in the headers. The spec uses this for
/// errors before any message.
fn trailers_only(status: Status) -> Response<BoxBody<Bytes, Infallible>> {
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, GRPC_CONTENT_TYPE)
        .header(GRPC_STATUS, status.code.as_u32().to_string());
    if !status.message.is_empty() {
        builder = builder.header(GRPC_MESSAGE, encode_message(&status.message));
    }
    builder
        .body(Full::new(Bytes::new()).boxed())
        .expect("status headers are valid")
}

fn plain(code: StatusCode) -> Response<BoxBody<Bytes, Infallible>> {
    Response::builder()
        .status(code)
        .body(Full::new(Bytes::new()).boxed())
        .expect("static response is valid")
}

/// Calls unary methods on one target. Safe to share between tasks.
pub struct Client {
    http: HyperClient<HttpConnector, Full<Bytes>>,
    authority: String,
    max_recv: usize,
}

impl Client {
    /// `addr` is `host:port`. The connection opens on the first call.
    pub fn new(addr: &str) -> Self {
        let http = HyperClient::builder(TokioExecutor::new())
            .http2_only(true)
            .build(HttpConnector::new());
        Self {
            http,
            authority: addr.to_string(),
            max_recv: DEFAULT_MAX_MESSAGE,
        }
    }

    /// Sets the largest response message accepted.
    pub fn with_max_message_size(mut self, n: usize) -> Self {
        self.max_recv = n;
        self
    }

    /// Makes one unary call. `timeout` is sent as `grpc-timeout` and enforced
    /// on this side too.
    pub async fn unary<Req, Resp>(
        &self,
        path: &str,
        req: &Req,
        timeout: Option<Duration>,
    ) -> Result<Resp, Status>
    where
        Req: MessageEncode,
        Resp: MessageDecode + Default,
    {
        let framed = encode_frame(&encode_msg(req)?);
        let mut builder = Request::post(format!("http://{}{}", self.authority, path))
            .header(CONTENT_TYPE, GRPC_CONTENT_TYPE)
            .header("te", "trailers");
        if let Some(d) = timeout {
            builder = builder.header(GRPC_TIMEOUT, encode_timeout(d));
        }
        let request = builder
            .body(Full::new(framed))
            .map_err(|e| Status::new(Code::Internal, format!("build request: {e}")))?;

        let call = async {
            let resp = self
                .http
                .request(request)
                .await
                .map_err(|e| Status::new(Code::Unavailable, format!("transport: {e}")))?;
            let (parts, body) = resp.into_parts();
            // A trailers-only response carries the status in the headers.
            if let Some(status) = status_from(&parts.headers)
                && status.code != Code::Ok
            {
                return Err(status);
            }
            let collected = Limited::new(body, self.max_recv + FRAME_HEADER_LEN)
                .collect()
                .await
                .map_err(|e| {
                    if e.downcast_ref::<http_body_util::LengthLimitError>()
                        .is_some()
                    {
                        Status::new(Code::ResourceExhausted, "response message too large")
                    } else {
                        Status::new(Code::Unavailable, "failed to read response")
                    }
                })?;
            let trailers = collected.trailers().cloned().unwrap_or_default();
            if let Some(status) = status_from(&trailers)
                && status.code != Code::Ok
            {
                return Err(status);
            }
            let data = collected.to_bytes();
            let payload = decode_frame(&data, self.max_recv)?;
            decode_msg(payload)
        };

        match timeout {
            Some(d) => tokio::time::timeout(d, call)
                .await
                .unwrap_or_else(|_| Err(Status::new(Code::DeadlineExceeded, "deadline exceeded"))),
            None => call.await,
        }
    }
}

/// Splits a body into its gRPC frames. Strict: a truncated frame, a compressed
/// frame, one message over `max_each`, or a body over `max_total` is an error.
/// An empty body is zero messages.
pub fn decode_frames(buf: &[u8], max_each: usize, max_total: usize) -> Result<Vec<&[u8]>, Status> {
    if buf.len() > max_total {
        return Err(Status::new(Code::ResourceExhausted, "response too large"));
    }
    let mut out = Vec::new();
    let mut rest = buf;
    while !rest.is_empty() {
        if rest.len() < FRAME_HEADER_LEN {
            return Err(Status::new(Code::Internal, "truncated frame header"));
        }
        if rest[0] != 0 {
            return Err(Status::new(
                Code::Unimplemented,
                "compressed messages are not supported",
            ));
        }
        let n = u32::from_be_bytes([rest[1], rest[2], rest[3], rest[4]]) as usize;
        if n > max_each {
            return Err(Status::new(Code::ResourceExhausted, "message too large"));
        }
        let end = FRAME_HEADER_LEN + n;
        if rest.len() < end {
            return Err(Status::new(Code::Internal, "truncated frame"));
        }
        out.push(&rest[FRAME_HEADER_LEN..end]);
        rest = &rest[end..];
    }
    Ok(out)
}

/// Checks outgoing request metadata before anything is sent. Keys must be
/// lowercase ASCII and unique. Reserved keys are refused: `te`, `content-type`,
/// `grpc-*` and binary (`-bin`) keys. Values must be visible ASCII or space,
/// which excludes CR and LF.
pub fn validate_metadata(pairs: &[(&str, &str)]) -> Result<(), Status> {
    let mut seen = std::collections::HashSet::new();
    for (key, value) in pairs {
        let key_ok = !key.is_empty()
            && key.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b'.')
            });
        if !key_ok {
            return Err(Status::new(
                Code::InvalidArgument,
                format!("invalid metadata key `{key}`"),
            ));
        }
        if key.starts_with("grpc-")
            || key.ends_with("-bin")
            || matches!(*key, "te" | "content-type")
        {
            return Err(Status::new(
                Code::InvalidArgument,
                format!("reserved metadata key `{key}`"),
            ));
        }
        if !seen.insert(*key) {
            return Err(Status::new(
                Code::InvalidArgument,
                format!("duplicate metadata key `{key}`"),
            ));
        }
        if !value.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
            return Err(Status::new(
                Code::InvalidArgument,
                format!("metadata `{key}` must be visible ASCII"),
            ));
        }
    }
    Ok(())
}

impl Client {
    /// Sends every request message in one body, then reads every response message.
    ///
    /// This is not bidirectional streaming. Nothing is interleaved, and there is no
    /// backpressure, so the whole request is held in memory. Metadata is validated
    /// before anything is sent. `max_recv` caps each response message and the whole
    /// response body.
    pub async fn call_buffered(
        &self,
        path: &str,
        metadata: &[(&str, &str)],
        requests: &[Vec<u8>],
        timeout: Option<Duration>,
    ) -> Result<Vec<Vec<u8>>, Status> {
        validate_metadata(metadata)?;
        let mut body = BytesMut::new();
        for r in requests {
            body.put_slice(&encode_frame(r));
        }

        let mut builder = Request::post(format!("http://{}{}", self.authority, path))
            .header(CONTENT_TYPE, GRPC_CONTENT_TYPE)
            .header("te", "trailers");
        for (key, value) in metadata {
            builder = builder.header(*key, *value);
        }
        if let Some(d) = timeout {
            builder = builder.header(GRPC_TIMEOUT, encode_timeout(d));
        }
        let request = builder
            .body(Full::new(body.freeze()))
            .map_err(|e| Status::new(Code::InvalidArgument, format!("build request: {e}")))?;

        let call = async {
            let resp = self
                .http
                .request(request)
                .await
                .map_err(|e| Status::new(Code::Unavailable, format!("transport: {e}")))?;
            let (parts, body) = resp.into_parts();
            // A status in the headers ends the call, before any body is read.
            if let Some(status) = status_from(&parts.headers)
                && status.code != Code::Ok
            {
                return Err(status);
            }
            let collected = Limited::new(body, self.max_recv)
                .collect()
                .await
                .map_err(|e| {
                    if e.downcast_ref::<http_body_util::LengthLimitError>()
                        .is_some()
                    {
                        Status::new(Code::ResourceExhausted, "response too large")
                    } else {
                        Status::new(Code::Unavailable, "failed to read response")
                    }
                })?;
            // Errors arrive in the trailers, possibly after zero messages. Check
            // them before parsing any frame.
            let trailers = collected.trailers().cloned().unwrap_or_default();
            if let Some(status) = status_from(&trailers)
                && status.code != Code::Ok
            {
                return Err(status);
            }
            let data = collected.to_bytes();
            let frames = decode_frames(&data, self.max_recv, self.max_recv)?;
            Ok(frames.into_iter().map(<[u8]>::to_vec).collect())
        };

        match timeout {
            Some(d) => tokio::time::timeout(d, call)
                .await
                .unwrap_or_else(|_| Err(Status::new(Code::DeadlineExceeded, "deadline exceeded"))),
            None => call.await,
        }
    }
}

/// Reads grpc-status and grpc-message from a header or trailer map. `None` if
/// the map has no grpc-status.
fn status_from(map: &HeaderMap) -> Option<Status> {
    let raw = map.get(&GRPC_STATUS)?.to_str().ok()?;
    let code = raw
        .parse::<u32>()
        .ok()
        .and_then(Code::from_u32)
        .unwrap_or(Code::Unknown);
    let message = map
        .get(&GRPC_MESSAGE)
        .and_then(|v| v.to_str().ok())
        .map(decode_message)
        .unwrap_or_default();
    Some(Status { code, message })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_wire_format_matches_go() {
        assert_eq!(&encode_frame(b"hi")[..], &[0, 0, 0, 0, 2, b'h', b'i']);
    }

    #[test]
    fn frame_rejects_compression_and_oversize() {
        assert_eq!(
            decode_frame(&[1, 0, 0, 0, 0], 10).unwrap_err().code,
            Code::Unimplemented
        );
        assert_eq!(
            decode_frame(&[0, 0, 0, 0, 20], 10).unwrap_err().code,
            Code::ResourceExhausted
        );
    }

    #[test]
    fn frame_rejects_trailing_bytes() {
        assert!(decode_frame(&[0, 0, 0, 0, 1, b'a', b'b'], 10).is_err());
    }

    #[test]
    fn timeout_round_trips_without_extending() {
        for d in [
            Duration::from_nanos(1),
            Duration::from_millis(100),
            Duration::from_secs(5),
        ] {
            let s = encode_timeout(d);
            assert!(s.len() <= 9, "{s}");
            assert!(parse_timeout(&s).unwrap() <= d, "{s}");
        }
        assert_eq!(encode_timeout(Duration::from_millis(100)), "100000u");
        assert!(parse_timeout("12X").is_none());
        assert!(parse_timeout("123456789m").is_none());
    }

    #[test]
    fn decode_frames_reads_many_messages() {
        let mut buf = encode_frame(b"a").to_vec();
        buf.extend_from_slice(&encode_frame(b"bb"));
        let frames = decode_frames(&buf, 10, 100).unwrap();
        assert_eq!(frames, vec![&b"a"[..], &b"bb"[..]]);
        assert!(decode_frames(&[], 10, 100).unwrap().is_empty());
    }

    #[test]
    fn decode_frames_is_strict() {
        assert_eq!(
            decode_frames(&[0, 0, 0, 0, 3, b'a'], 10, 100)
                .unwrap_err()
                .code,
            Code::Internal
        );
        assert_eq!(
            decode_frames(&[0, 0], 10, 100).unwrap_err().code,
            Code::Internal
        );
        assert_eq!(
            decode_frames(&[1, 0, 0, 0, 0], 10, 100).unwrap_err().code,
            Code::Unimplemented
        );
        assert_eq!(
            decode_frames(&[0, 0, 0, 0, 20], 10, 100).unwrap_err().code,
            Code::ResourceExhausted
        );
        let big = encode_frame(&[0u8; 20]);
        assert_eq!(
            decode_frames(&big, 100, 10).unwrap_err().code,
            Code::ResourceExhausted
        );
    }

    #[test]
    fn metadata_rules() {
        assert!(validate_metadata(&[("authorization", "Bearer abc"), ("x-trace", "1")]).is_ok());
        let bad: Vec<Vec<(&str, &str)>> = vec![
            vec![("te", "x")],
            vec![("content-type", "x")],
            vec![("grpc-timeout", "1S")],
            vec![("x-data-bin", "AA==")],
            vec![(":path", "/")],
            vec![("X-Upper", "1")],
            vec![("", "1")],
            vec![("x-a", "1"), ("x-a", "2")],
            vec![("x-a", "line\nbreak")],
            vec![("x-a", "tab\there")],
        ];
        for case in bad {
            assert_eq!(
                validate_metadata(&case).unwrap_err().code,
                Code::InvalidArgument,
                "{case:?}"
            );
        }
    }

    #[test]
    fn message_escaping_round_trips() {
        for m in ["plain", "héllo 100% ok", "line\nbreak", ""] {
            assert_eq!(decode_message(&encode_message(m)), m);
        }
    }
}
