// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Transports and link framing for tpt-hands.
//!
//! `Transport` is the datagram interface that serial, UDP and CAN implement.
//! `Link` adds sequencing and loss counting on top, and `Heartbeat` tracks peer
//! liveness. `Loopback` and `FaultyTransport` make it testable without hardware.

#![forbid(unsafe_code)]

pub mod fault;
pub mod link;
pub mod transport;
pub mod udp;

pub use fault::FaultyTransport;
pub use link::{Heartbeat, Link, LinkError, LinkStats};
pub use transport::{Closed, Loopback, Transport};
pub use udp::UdpTransport;
