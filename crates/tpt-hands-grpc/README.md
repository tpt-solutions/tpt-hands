<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 TPT Solutions -->

# tpt-hands-grpc

gRPC over HTTP/2 for the Rust host, with micropb messages. It is the Rust
counterpart of `go/tpt-grpc-lite`, and the two interoperate.

Built on hyper and micropb. tonic is not used: tonic 0.14 depends on
`sync_wrapper`, which is Apache-2.0 only and banned by the licence policy.

## Scope

- Unary calls, both as server handlers and as client calls
- Status in trailers, with the message percent-encoded
- Deadlines: the client sends `grpc-timeout` and the server enforces it. The
  client enforces it too.
- Message size limits, checked while reading, so an oversize body is refused
  without being buffered in full
- Cleartext HTTP/2 (h2c-style, prior knowledge)

## Buffered calls and request metadata

`Client::call_buffered` sends several request messages in one body, then reads
several response messages. It is not bidirectional streaming: nothing is
interleaved and there is no backpressure, so the whole request is held in memory.
Request metadata is validated before anything is sent. Keys must be lowercase
ASCII and unique. `te`, `content-type`, `grpc-*` and `-bin` keys are refused, and
values must be visible ASCII.

Errors can arrive in the headers (a trailers-only status) or in the trailers
after zero or more messages. `call_buffered` checks both before it parses any
frame, and a partial result is discarded when an error follows.

## Not yet

- Streaming (server, client and bidi)
- Server-side metadata and response metadata
- TLS and mTLS. The bearer token therefore travels in cleartext: use loopback or
  a trusted network.
- Compression and interceptors

## Interop

`cargo test -p tpt-hands-grpc` covers the Rust side. The Go test
`go/tpt-hands-orchestrator/internal/interop/rust_interop_test.go` runs both
directions: `ack-server` against the Go client, and `ack-client` against a
`tpt-grpc-lite` server.
