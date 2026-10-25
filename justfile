# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 TPT Solutions

# Common developer commands. Install just: https://github.com/casey/just

default: check

# Format check, lint and tests (same as CI)
check: fmt lint test

fmt:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

doc:
    cargo doc --workspace --no-deps

# Licence, ban and source policy (needs cargo-deny)
deny:
    cargo deny check licenses bans sources

# Regenerate the Rust protobuf code from proto/. Needs protoc on PATH, or PROTOC set.
proto:
    cargo run -q -p proto-gen

# Regenerate the Go protobuf code from proto/. Needs protoc and protoc-gen-go on PATH.
proto-go:
    mkdir -p go/tpt-hands-orchestrator/internal/gen/tpthandsv1
    protoc -Iproto --go_out=go/tpt-hands-orchestrator/internal/gen/tpthandsv1 --go_opt=paths=source_relative --go_opt='Mtpt_hands.proto=github.com/tpt-solutions/tpt-hands/go/tpt-hands-orchestrator/internal/gen/tpthandsv1;tpthandsv1' proto/tpt_hands.proto

# Regenerate the third-party licence notice for the Rust workspace.
third-party:
    cargo about generate about.hbs -o THIRD_PARTY_LICENSES.md

# Breaking-change check for the proto against its baseline. See docs/PROTO_COMPATIBILITY.md.
proto-compat:
    cd tools/proto-compat && python -m unittest test_proto_compat
    python tools/proto-compat/proto_compat.py proto/baseline/tpt_hands.proto proto/tpt_hands.proto
