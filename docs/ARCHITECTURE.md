<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 TPT Solutions -->

# Architecture overview

This describes the repository as it is now. [DESIGN.md](../DESIGN.md) holds the
intended design, and [todo.md](../todo.md) holds the status of each item. Where
the two differ, this file says so.

## Layers

tpt-hands is software only. It sits between the cognitive and physiological
layers (tpt-cerebrum, tpt-soma, tpt-chrysalis) and the physical end-effectors.

| Layer | Where | State |
| --- | --- | --- |
| Shared SDK: traits and types | `crates/tpt-hands-sdk` | Done. `EndEffector`, capabilities, telemetry, the `SimulatedHand` mock. |
| Wire types | `proto/`, `crates/tpt-hands-proto` | Provisional. Generated Rust (micropb) and Go (protobuf-go) bindings. Reconciled against tpt-link, not yet against the real schemas. |
| Frame and transports | `tpt-link` (separate repo, pinned); `crates/tpt-hands-proto` re-exports its frame | Host side done for serial, UDP and CAN framing. Firmware sides pending. |
| Edge reflexes | `crates/tpt-hands-edge` (`no_std`) | Host-testable logic: pathway monitor, route pick, grip reflex, frame parse. MCU bring-up pending. |
| Power handshake | `crates/tpt-hands-handshake` | Reference state machine with Ed25519 signatures and replay counters. Not audited. Formal proofs pending. |
| Morphogenesis simulation | `sim/morphogenesis_plugin` | In-house single-joint model with healing, reinforcement and a scripted baseline agent. |
| Ecosystem adapters | `crates/tpt-hands-soma-adapter`, `crates/tpt-hands-chrysalis-adapter` | Chrysalis: mapping, lifecycle, and an HTTP client tested against a labelled mock. Soma: placeholder, blocked on tpt-soma. |
| Rust gRPC | `crates/tpt-hands-grpc` | Unary over cleartext HTTP/2 with hyper and micropb. Streaming, metadata and TLS pending. |
| Go gRPC | `go/tpt-grpc-lite` | Unary and streaming over HTTP/2, TLS and mTLS, deadlines, metadata. Separate module. |
| Go orchestrator | `go/tpt-hands-orchestrator` | Health and version endpoints, config. Routing, device registry and the Cerebrum intent path are not built. |

## Data flow, as built

```
tpt-cerebrum ──intent──▶ orchestrator (Go)           not built
                              │
                              ▼
                        tpt-grpc-lite / tpt-hands-grpc
                              │  Ack, Telemetry, Handshake (protobuf over gRPC)
                              ▼
            tpt-chrysalis ◀── chrysalis adapter ── MorphogenesisRequest
            (REST, proposed interface; see ECOSYSTEM_INTEGRATION.md)
                              │
                              ▼
            end-effector ◀── tpt-link frames (CRC, sequence) ── edge reflexes
```

The orchestrator-to-edge link and the Cerebrum intent path are the big gaps.
Everything above the wire is tested in isolation.

## Decisions that shape the layout

- **Licence policy.** Permissive only. `cargo deny` enforces it for the Rust
  workspace, and `THIRD_PARTY_LICENSES.md` records it. Apache-2.0-only crates
  are banned. That is why tonic was dropped, since it depends on `sync_wrapper`.
- **No physics engine.** The morphogenesis model is in-house, so the licence
  check stays simple.
- **Synchronous SDK.** The SDK has no async runtime. Async appears only in the
  transports and the gRPC layers.
- **Mock-first adapters.** Chrysalis and soma have no real device endpoints
  yet. The adapters are built against labelled proposals, so they can be tested
  now and changed when the owners decide.
- **Two gRPC implementations, one wire format.** Go and Rust each implement the
  subset they need. The interop tests run both directions against each other and
  against grpc-core.

## Where the risks are

- The protobuf schema is a provisional transcription, not the real tpt-link
  schema.
- The power handshake is not audited, and its formal proofs are not done.
- Nothing has run on hardware. Timing numbers are host numbers.
- The soma and chrysalis APIs do not match what the design assumed. See
  [ECOSYSTEM_INTEGRATION.md](ECOSYSTEM_INTEGRATION.md).
