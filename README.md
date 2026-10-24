# tpt-hands

Peripheral nervous system and morphological edge layer for the TPT robotics
ecosystem. tpt-hands is the hardware-agnostic bridge between high-level intent
(tpt-cerebrum), physiological state (tpt-soma) and microfluidic actuation
(tpt-chrysalis), and the physical end-effectors it drives.

Status: in development. Phases 0 to 2 are done, except where [todo.md](todo.md)
notes pending work. Those cover the SDK traits and `SimulatedHand` mock, the
link frame format and transports (via tpt-link), the on-device reflexes, and
the soma and chrysalis adapters. Smart power and formal proofs (Phase 3),
sim2real morphogenesis (Phase 4) and release hardening (Phase 5) are still to
do. See [DESIGN.md](DESIGN.md) for the specification,
[todo.md](todo.md) for the phased plan and
[docs/ECOSYSTEM_INTEGRATION.md](docs/ECOSYSTEM_INTEGRATION.md) for how the
adapters map onto the real tpt-soma and tpt-chrysalis APIs.

## Layout

| Path | Purpose |
| --- | --- |
| `crates/tpt-hands-sdk` | Public Rust SDK: traits, core types, mock drivers |
| `crates/tpt-hands-proto` | Protobuf wire definitions and the link frame format |
| [tpt-link](https://github.com/tpt-solutions/tpt-link) (separate repo, dependency) | Shared wire layer: frame format, transports, `Link`, heartbeat, reconnect, CAN and core schema. |
| `crates/tpt-hands-edge` | ESP32-C6 and STM32G4 firmware (`no_std`) |
| `crates/tpt-hands-soma-adapter` | Syncs physiological state to tpt-soma |
| `crates/tpt-hands-chrysalis-adapter` | Sends morphogenesis requests to tpt-chrysalis |

Also in the tree: `go/tpt-hands-orchestrator` (Go service bridging tpt-swarm,
thin for now), `proto/tpt_hands.proto` (the wire schema) and `tools/proto-gen`
(regenerates the committed protobuf code). Planned: `sim/morphogenesis_plugin`
(in-house soft-body model).

## Development

Requires Rust (version pinned in `rust-toolchain.toml`). Common tasks are in the
`justfile`:

```sh
just check   # fmt, clippy (warnings are errors), tests
just deny    # licence and dependency policy (needs cargo-deny)
just proto   # regenerate protobuf code from proto/ (needs protoc on PATH, or PROTOC set)
```

The generated protobuf code is committed, so ordinary builds do not need
`protoc`. Only regenerating it does. CI fails if the committed code is stale.

## Licence

Dual-licensed under either of

- [MIT licence](LICENSE-MIT)
- [Apache licence, version 2.0](LICENSE-APACHE)

at your option. Copyright (c) 2026 TPT Solutions.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this work is licensed as above, without any additional terms.
See [CONTRIBUTING.md](CONTRIBUTING.md).

Dependencies must be permissive. Apache-2.0-only dependencies are not allowed;
the policy is enforced by `deny.toml`.
