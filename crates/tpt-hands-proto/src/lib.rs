// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Protobuf wire definitions for tpt-hands.
//!
//! Generated message types (`generated`, from `proto/tpt_hands.proto`, via
//! micropb, no prost) and the link frame format that every transport carries.
//!
//! Features: `std` (default) and `alloc` enable the encoder. Without them the
//! crate is `no_std` and only decoding is available, which is what the edge
//! firmware uses.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

// The frame format is owned by tpt-link. Re-exported so existing paths keep working.
pub use tpt_link::frame;
#[cfg(feature = "alloc")]
pub mod generated;
