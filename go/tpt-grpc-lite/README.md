<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 TPT Solutions -->

# tpt-grpc-lite

A small gRPC client and server over HTTP/2, for Go. It is licence-clean: it uses
`golang.org/x/net` (BSD-3) and `google.golang.org/protobuf` (BSD-3), and no
`grpc-go` or `prost`, which are Apache-2.0-only.

Module: `github.com/tpt-solutions/tpt-hands/go/tpt-grpc-lite`. Requires Go 1.26
(pulled in by the current `golang.org/x/net`).

## Decision: separate module, same repo

It lives in this repo as its own Go module, so it can be reused by other TPT
repos and split out later without a history rewrite.

## What is in

- Unary, server-streaming, client-streaming and bidi-streaming calls
- The 5-byte length-prefixed message frame, and the `grpc-status` /
  `grpc-message` trailers, with the message percent-encoded
- Deadlines (`grpc-timeout`), both ways: the client sends the remaining time
  and the server enforces it
- Cancellation: cancelling the client context cancels the call on the server
- Metadata in both directions, including `-bin` keys (base64)
- TLS through `Server.ServeTLS` and `WithTLS`. mTLS is set in the server's
  `tls.Config` (`ClientAuth`, `ClientCAs`).
- Message size limits on send and receive (default 4 MiB)
- Reconnect: a new connection is dialled on the next call after a drop. Tested
  across a server restart.
- Keepalive pings (`WithKeepalive`). A test freezes a TCP proxy, so the peer
  goes silent without closing, and checks that the call fails well before its
  30 s deadline. The test fails when keepalive is removed.

## What is out

Compression, retries, interceptors, reflection, health checking, load balancing,
and name resolution. These are not planned for v1.

## Interop

`interop_test.go` checks both directions against grpc-core, the C
implementation behind Python's `grpc` package, and skips when Python with
`grpcio` is missing. Scripts are in `interop/`. Covered: unary, error status
with a non-ASCII message, unknown method, metadata including `-bin`, deadlines,
and all three streaming shapes.

## Not yet verified

- **Rust tonic and the real tpt-soma, tpt-chrysalis and tpt-swarm servers.** Not
  tested. tonic needs a prost-free codec spike first, and the real servers need
  their protos and a live endpoint.
- **Race detector.** `go test -race` does not run on this Windows machine. The
  clang-based toolchain from PyPI (`ziglang`) builds the package, but the
  ThreadSanitizer runtime fails at startup with DLL-not-found (0xc0000135), even
  with a generated `synchronization` import library and a static link. Run it
  on CI or a machine with a native gcc or MSVC toolchain.

## Running

```
cd go/tpt-grpc-lite
go test ./...
go test -run xxx -bench . ./...
```

Benchmarks run on loopback, so they measure the stack and not a real network.
