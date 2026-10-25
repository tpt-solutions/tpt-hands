// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Phase 0 spike: tonic with a codec backed by micropb, and no prost.
//!
//! tonic's own codegen assumes prost. This crate writes the service glue by
//! hand, so the wire types are the micropb messages from `tpt-hands-proto`.
//! The spike covers one unary method, `tpt.hands.v1.Ack/Acknowledge`. It takes a
//! `MorphogenesisRequest` and returns a `MorphogenesisAck`.

use std::convert::Infallible;
use std::marker::PhantomData;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Buf;
use micropb::{MessageDecode, MessageEncode, PbEncoder};
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::codegen::{Body, BoxFuture, StdError, http};
use tonic::server::{NamedService, UnaryService};
use tonic::{Request, Response, Status};

pub use tpt_hands_proto::generated::v1::{MorphogenesisAck, MorphogenesisRequest};

/// Full method path the spike serves.
pub const ACKNOWLEDGE_PATH: &str = "/tpt.hands.v1.Ack/Acknowledge";

/// Codec that frames micropb messages for tonic. tonic adds the 5-byte gRPC
/// prefix, so this only converts between messages and bytes.
pub struct MicropbCodec<E, D> {
    _types: PhantomData<fn() -> (E, D)>,
}

impl<E, D> Default for MicropbCodec<E, D> {
    fn default() -> Self {
        Self {
            _types: PhantomData,
        }
    }
}

impl<E, D> Codec for MicropbCodec<E, D>
where
    E: MessageEncode + Send + 'static,
    D: MessageDecode + Default + Send + 'static,
{
    type Encode = E;
    type Decode = D;
    type Encoder = MicropbEncoder<E>;
    type Decoder = MicropbDecoder<D>;

    fn encoder(&mut self) -> Self::Encoder {
        MicropbEncoder { _msg: PhantomData }
    }

    fn decoder(&mut self) -> Self::Decoder {
        MicropbDecoder { _msg: PhantomData }
    }
}

pub struct MicropbEncoder<E> {
    _msg: PhantomData<fn() -> E>,
}

impl<E: MessageEncode> Encoder for MicropbEncoder<E> {
    type Item = E;
    type Error = Status;

    fn encode(&mut self, item: E, dst: &mut EncodeBuf<'_>) -> Result<(), Status> {
        let mut enc = PbEncoder::new(Vec::new());
        item.encode(&mut enc)
            .map_err(|e| Status::internal(format!("micropb encode: {e:?}")))?;
        bytes::BufMut::put_slice(dst, &enc.into_writer());
        Ok(())
    }
}

pub struct MicropbDecoder<D> {
    _msg: PhantomData<fn() -> D>,
}

impl<D: MessageDecode + Default> Decoder for MicropbDecoder<D> {
    type Item = D;
    type Error = Status;

    fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<D>, Status> {
        // tonic hands over exactly one message's bytes for a unary call.
        let bytes = src.copy_to_bytes(src.remaining());
        let mut msg = D::default();
        msg.decode_from_bytes(&bytes)
            .map_err(|e| Status::invalid_argument(format!("micropb decode: {e:?}")))?;
        Ok(Some(msg))
    }
}

/// Business logic for the Ack service. Implement this in the orchestrator.
pub trait Acknowledger: Send + Sync + 'static {
    fn acknowledge(&self, req: MorphogenesisRequest) -> Result<MorphogenesisAck, Status>;
}

/// The spike's implementation: approves any request that names a tool.
#[derive(Default)]
pub struct ApproveAll;

impl Acknowledger for ApproveAll {
    fn acknowledge(&self, req: MorphogenesisRequest) -> Result<MorphogenesisAck, Status> {
        if req.tool_id.is_empty() {
            return Err(Status::invalid_argument("tool_id is required"));
        }
        Ok(MorphogenesisAck {
            request_id: req.request_id,
            approved: true,
            ..Default::default()
        })
    }
}

/// Service for `tpt.hands.v1.Ack`. Built around an `Acknowledger`.
pub struct AckServer<A> {
    inner: Arc<A>,
}

impl<A> AckServer<A> {
    pub fn new(inner: A) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

impl<A> Clone for AckServer<A> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<A> NamedService for AckServer<A> {
    const NAME: &'static str = "tpt.hands.v1.Ack";
}

struct AckMethod<A>(Arc<A>);

impl<A: Acknowledger> UnaryService<MorphogenesisRequest> for AckMethod<A> {
    type Response = MorphogenesisAck;
    type Future = BoxFuture<Response<MorphogenesisAck>, Status>;

    fn call(&mut self, request: Request<MorphogenesisRequest>) -> Self::Future {
        let inner = self.0.clone();
        Box::pin(async move {
            let ack = inner.acknowledge(request.into_inner())?;
            Ok(Response::new(ack))
        })
    }
}

impl<A, B> tonic::codegen::Service<http::Request<B>> for AckServer<A>
where
    A: Acknowledger,
    B: Body + Send + 'static,
    B::Error: Into<StdError> + Send + 'static,
{
    type Response = http::Response<tonic::body::Body>;
    type Error = Infallible;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::Request<B>) -> Self::Future {
        if req.uri().path() == ACKNOWLEDGE_PATH {
            let method = AckMethod(self.inner.clone());
            Box::pin(async move {
                let codec: MicropbCodec<MorphogenesisAck, MorphogenesisRequest> =
                    MicropbCodec::default();
                let mut grpc = tonic::server::Grpc::new(codec);
                Ok(grpc.unary(method, req).await)
            })
        } else {
            // Trailers-only UNIMPLEMENTED (grpc-status 12), as the spec describes.
            Box::pin(async {
                let resp = http::Response::builder()
                    .status(200)
                    .header("content-type", "application/grpc")
                    .header("grpc-status", "12")
                    .body(tonic::body::Body::default())
                    .expect("static response is valid");
                Ok(resp)
            })
        }
    }
}

/// Serves `svc` on `lis` over HTTP/2 until the listener fails. This replaces
/// tonic's `Server::add_service`, which would need the `router` feature.
pub async fn serve<A: Acknowledger>(
    lis: tokio::net::TcpListener,
    svc: AckServer<A>,
) -> std::io::Result<()> {
    use hyper_util::rt::{TokioExecutor, TokioIo};
    use hyper_util::server::conn::auto::Builder;
    use hyper_util::service::TowerToHyperService;

    loop {
        let (stream, _peer) = lis.accept().await?;
        let svc = TowerToHyperService::new(svc.clone());
        tokio::spawn(async move {
            // A connection that fails mid-call only affects that connection.
            let _ = Builder::new(TokioExecutor::new())
                .http2_only()
                .serve_connection(TokioIo::new(stream), svc)
                .await;
        });
    }
}

/// Client for `tpt.hands.v1.Ack`.
pub struct AckClient {
    grpc: tonic::client::Grpc<tonic::transport::Channel>,
}

impl AckClient {
    pub fn new(channel: tonic::transport::Channel) -> Self {
        Self {
            grpc: tonic::client::Grpc::new(channel),
        }
    }

    pub async fn acknowledge(
        &mut self,
        req: MorphogenesisRequest,
    ) -> Result<MorphogenesisAck, Status> {
        self.grpc
            .ready()
            .await
            .map_err(|e| Status::unavailable(format!("not ready: {e}")))?;
        let path = tonic::codegen::http::uri::PathAndQuery::from_static(ACKNOWLEDGE_PATH);
        let codec: MicropbCodec<MorphogenesisRequest, MorphogenesisAck> = MicropbCodec::default();
        let resp = self.grpc.unary(Request::new(req), path, codec).await?;
        Ok(resp.into_inner())
    }
}
