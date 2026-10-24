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
- Keepalive pings (`WithKeepalive`). The option is set, but I have not tested
  it against a dead peer.

## What is out

Compression, retries, interceptors, reflection, health checking, load balancing,
and name resolution. These are not planned for v1.

## Not yet verified

- **Interop with the Rust tonic stack and the real tpt-soma, tpt-chrysalis and
  tpt-swarm servers.** Tests here are Go-to-Go only. Interop is a todo.md Phase 0
  item.
- **Race detector.** Not run: this machine has no C compiler, which `-race`
  needs. Run `go test -race ./...` on CI or a machine with gcc.

## Running

```
cd go/tpt-grpc-lite
go test ./...
go test -run xxx -bench . ./...
```

Benchmarks run on loopback, so they measure the stack and not a real network.
