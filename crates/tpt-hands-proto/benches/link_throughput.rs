// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Throughput of the tpt-link frame path over UDP on loopback: `Link::send` on
//! one side, `Link::poll` on the other, with the frame CRC and sequence checks
//! in between. Loopback has no real radio, serial or CAN latency, so these are
//! upper bounds for the host stack only.
//!
//! Run with `cargo bench -p tpt-hands-proto`.

use std::hint::black_box;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use tpt_link::link::Link;
use tpt_link::udp::UdpTransport;

/// Messages sent before the receiver is drained. Keeps the socket buffer from
/// overflowing on loopback.
const BATCH: usize = 32;

/// Wall-clock time spent on each payload size.
const RUN_FOR: Duration = Duration::from_millis(800);

/// How long to wait for a batch to arrive before counting it as lost.
const DRAIN_WAIT: Duration = Duration::from_millis(50);

/// Binds a receiver, then a sender that targets it. The benchmark is one-way,
/// so the receiver never sends and its own peer is a placeholder.
fn pair() -> (Link<UdpTransport>, Link<UdpTransport>) {
    let placeholder: SocketAddr = "127.0.0.1:9".parse().unwrap();
    let rx = UdpTransport::bind("127.0.0.1:0", placeholder).unwrap();
    let rx_addr = rx.local_addr().unwrap();
    let tx = UdpTransport::bind("127.0.0.1:0", rx_addr).unwrap();
    (Link::new(tx), Link::new(rx))
}

struct Sample {
    payload: usize,
    sent: u64,
    received: u64,
    elapsed: Duration,
}

fn run(payload_len: usize) -> Sample {
    let (mut tx, mut rx) = pair();
    let payload = vec![0xA5u8; payload_len];

    let mut sent = 0u64;
    let mut received = 0u64;
    let start = Instant::now();
    while start.elapsed() < RUN_FOR {
        let mut batch_sent = 0u64;
        for _ in 0..BATCH {
            tx.send(black_box(&payload)).unwrap();
            batch_sent += 1;
        }
        sent += batch_sent;

        // Drain the batch. A datagram that does not arrive within DRAIN_WAIT is
        // counted as lost, so a lossy run ends instead of hanging.
        let mut batch_received = 0u64;
        let drain_start = Instant::now();
        while batch_received < batch_sent && drain_start.elapsed() < DRAIN_WAIT {
            for p in rx.poll().unwrap() {
                debug_assert_eq!(p.len(), payload_len);
                black_box(p);
                batch_received += 1;
            }
        }
        received += batch_received;
    }
    Sample {
        payload: payload_len,
        sent,
        received,
        elapsed: start.elapsed(),
    }
}

fn main() {
    println!("tpt-link over UDP loopback, batch of {BATCH}, {RUN_FOR:?} per size");
    println!(
        "  {:>8}  {:>12}  {:>10}  {:>10}  {:>10}",
        "payload", "msgs/s", "MB/s", "delivered", "loss"
    );
    for &size in &[16usize, 128, 1024] {
        let r = run(size);
        let secs = r.elapsed.as_secs_f64();
        let msgs = r.received as f64 / secs;
        let mbps = msgs * r.payload as f64 / 1e6;
        let loss = if r.sent == 0 {
            0.0
        } else {
            100.0 * (r.sent - r.received) as f64 / r.sent as f64
        };
        println!(
            "  {:>7}B  {:>12.0}  {:>10.2}  {:>10}  {:>9.2}%",
            r.payload, msgs, mbps, r.received, loss
        );
    }
}
