// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Protobuf wire definitions for tpt-hands.
//!
//! Generated message types (micropb-gen, no prost) are added once the real
//! tpt-link schema is imported. Until then this crate holds the link frame
//! format that every transport carries.
//!
//! Features: `std` (default) and `alloc` enable the encoder. Without them the
//! crate is `no_std` and only decoding is available, which is what the edge
//! firmware uses.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod frame;
#[cfg(feature = "alloc")]
pub mod wire;
