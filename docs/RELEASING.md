<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 TPT Solutions -->

# Releasing

This is the release process for the Rust crates and the Go modules. Nothing has
been released yet, and v1.0 needs the hardware and formal items in `todo.md`.

## Versions

- Rust crates share one version through `[workspace.package]`, so they move
  together.
- The Go orchestrator and `tpt-grpc-lite` are separate modules, each with its own
  tags. Go tags are `go/tpt-grpc-lite/vX.Y.Z` and the same form for the
  orchestrator.
- Before 1.0, a minor bump (0.x to 0.y) may break the API. A patch bump may not.
- From 1.0, the usual rules apply: breaking changes need a major bump.
- The wire format is versioned separately. It follows the rules in
  `docs/PROTO_COMPATIBILITY.md`, and a breaking proto change needs a major bump
  even before 1.0.

## Before tagging

Each of these must pass on the commit being tagged. CI runs most of them.

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo deny check licenses bans sources`
5. `just third-party` regenerates `THIRD_PARTY_LICENSES.md`, and the diff is
   committed.
6. `just proto-compat` passes. A proto change that breaks the baseline is a major
   change, and the baseline is updated only as part of that change.
7. For Go: `gofmt`, `go vet`, `go test` and the go-licenses check, for each
   module changed.
8. `CHANGELOG.md`: move the **Unreleased** entries under the new version, with the
   date.
9. The known limits in the changelog still hold. If one is fixed, say so.

## Publishing

- `tpt-hands-sdk` is the first crate intended for crates.io (todo.md, Phase 0).
  Dry-run first: `cargo publish -p tpt-hands-sdk --dry-run`.
- Crates marked `publish = false` are never published. That covers the
  integration crate, the examples, and anything that depends on a path-only crate.
- `tpt-link` is not published by this repo. Its version is pinned by commit.
- Do not publish a crate that depends on a git-only dependency.

## Tags and notes

- Tag the commit, not a branch: `git tag -a vX.Y.Z -m "..."`.
- Release notes are the changelog section, with the known limits kept in.

## Not yet decided

- Whether `tpt-hands-grpc` and `tpt-hands-chrysalis-adapter` are published at
  all, given they depend on the proposed chrysalis interface.
- The signing of release artifacts. Nothing is signed yet.
