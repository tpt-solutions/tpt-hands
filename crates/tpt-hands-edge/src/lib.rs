// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! tpt-edge firmware library (`no_std`).
//!
//! Reflex loops, pathway monitoring and the power handshake are added in
//! Phases 1 and 3 (see `todo.md`). Target-specific bring-up lives in
//! separate binaries built for the MCU target.

#![no_std]
