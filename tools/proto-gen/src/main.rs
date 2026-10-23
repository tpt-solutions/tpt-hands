// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Regenerates `crates/tpt-hands-proto/src/generated/` from `proto/`.
//!
//! Needs `protoc` on PATH, or the `PROTOC` environment variable set to its
//! path. Run it with `just proto`. The output is committed, so ordinary builds
//! do not need `protoc`. CI checks that the committed output is current.

use std::fs;
use std::io;
use std::path::PathBuf;

fn main() -> io::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let proto_dir = root.join("proto");
    let out_dir = root.join("crates/tpt-hands-proto/src/generated");
    fs::create_dir_all(&out_dir)?;

    let mut generator = micropb_gen::Generator::new();
    // Vec and String containers, which need the `alloc` feature of tpt-hands-proto.
    generator.use_container_alloc();
    // Format inside the generator, so the committed file is exactly what the
    // generator writes. CI compares against a fresh run.
    generator.format(true);
    generator.add_protoc_arg(format!("-I{}", proto_dir.display()));
    generator.compile_protos(
        &[proto_dir.join("tpt_hands.proto")],
        out_dir.join("tpt_hands.rs"),
    )
}
