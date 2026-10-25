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

## Not yet

- Streaming (server, client and bidi)
- Metadata
- TLS and mTLS
- Compression and interceptors

## Interop

`cargo test -p tpt-hands-grpc` covers the Rust side. The Go test
`go/tpt-hands-orchestrator/internal/interop/rust_interop_test.go` runs both
directions: `ack-server` against the Go client, and `ack-client` against a
`tpt-grpc-lite` server.
