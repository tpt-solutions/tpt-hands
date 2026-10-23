# tpt-hands

Peripheral nervous system and morphological edge layer for the TPT robotics
ecosystem. tpt-hands is the hardware-agnostic bridge between high-level intent
(tpt-cerebrum), physiological state (tpt-soma) and microfluidic actuation
(tpt-chrysalis), and the physical end-effectors it drives.

Status: early development (Phase 0). See [DESIGN.md](DESIGN.md) for the
specification and [todo.md](todo.md) for the phased plan.

## Layout

| Path | Purpose |
| --- | --- |
| `crates/tpt-hands-sdk` | Public Rust SDK: traits, core types, mock drivers |
| `crates/tpt-hands-proto` | Protobuf wire definitions and the link frame format |
| `crates/tpt-link` | Transports, sequenced `Link`, heartbeat, loopback and fault injection |
| `crates/tpt-hands-edge` | ESP32-C6 and STM32G4 firmware (`no_std`) |
| `crates/tpt-hands-soma-adapter` | Syncs physiological state to tpt-soma |
| `crates/tpt-hands-chrysalis-adapter` | Sends morphogenesis requests to tpt-chrysalis |

Planned: `go/tpt-hands-orchestrator` (Go service bridging tpt-swarm) and
`sim/morphogenesis_plugin` (in-house soft-body model).

## Development

Requires Rust (version pinned in `rust-toolchain.toml`). Common tasks are in the
`justfile`:

```sh
just check   # fmt, clippy (warnings are errors), tests
just deny    # licence and dependency policy (needs cargo-deny)
```

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
