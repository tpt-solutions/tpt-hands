<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 TPT Solutions -->

# grpc-spike

Phase 0 spike: tonic (Rust) with a codec backed by micropb, and no prost. It
also checks interop with `tpt-grpc-lite` (Go) in both directions.

## What it shows

- `MicropbCodec` implements `tonic::codec::Codec` over the micropb messages in
  `tpt-hands-proto`. tonic supplies the 5-byte framing.
- `AckServer` and `AckClient` implement the `tpt.hands.v1.Ack/Acknowledge` unary
  method by hand, since tonic's codegen assumes prost.
- `serve` runs the service over HTTP/2 with hyper, so tonic's `router` feature
  (which needs axum) is not used.
- `cargo tree` shows no prost in the tree.

Run the tests with `cargo test` from this directory. The Go interop test in
`go/tpt-hands-orchestrator/internal/interop` builds the two binaries and runs
them against the Go side.

## Licence finding: tonic conflicts with the policy

tonic 0.14 declares `sync_wrapper` as a hard dependency, and `sync_wrapper` is
Apache-2.0 only. The project policy (`deny.toml`, todo.md) bans Apache-2.0-only
crates. So tonic itself cannot meet the policy, with or without the `router`
feature.

This crate is therefore kept **out of the root workspace**, so the workspace
licence gate stays green. Options for the decision are recorded in `todo.md`,
Phase 0.

## Not covered

- Server streaming, client streaming, and bidi calls. Only unary is spiked.
- TLS.
