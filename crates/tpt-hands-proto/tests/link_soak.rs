// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Soak test: a long run over the tpt-link frame path with drops and corruption
//! injected throughout. It checks the accounting, and above all that a damaged
//! frame is never delivered as good data.
//!
//! Every message carries its own index and a pattern derived from it, so any
//! delivered payload can be checked on its own. The run is deterministic, so a
//! failure reproduces.

use tpt_link::link::Link;
use tpt_link::{FaultyTransport, Loopback};

/// xorshift64. Fixed seed, so the run is the same every time.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// A payload for message `index`: the index, then a length and a pattern that
/// depend on it.
fn payload_for(index: u32) -> Vec<u8> {
    let len = 4 + (index as usize * 7) % 200;
    let mut p = Vec::with_capacity(len);
    p.extend_from_slice(&index.to_le_bytes());
    for i in 4..len {
        p.push((index as usize).wrapping_mul(31).wrapping_add(i) as u8);
    }
    p
}

/// Checks that `p` is a payload that `payload_for` could have produced.
fn index_of(p: &[u8]) -> Option<u32> {
    if p.len() < 4 {
        return None;
    }
    let index = u32::from_le_bytes([p[0], p[1], p[2], p[3]]);
    (payload_for(index) == p).then_some(index)
}

#[test]
fn soak_with_drops_and_corruption_keeps_the_books_balanced() {
    const MESSAGES: u32 = 20_000;
    const BATCH: u32 = 16;

    let (a, b) = Loopback::pair();
    let mut tx = Link::new(FaultyTransport::new(a));
    let mut rx = Link::new(b);

    let mut rng = Rng(0x243F_6A88_85A3_08D3);
    let mut dropped = 0u64;
    let mut corrupted = 0u64;
    let mut delivered = Vec::new();

    for i in 0..MESSAGES {
        // Inject a burst of faults now and then. The last stretch is clean, so
        // a trailing gap is still visible to the receiver.
        if i < MESSAGES - 1_000 && rng.below(50) == 0 {
            if rng.below(2) == 0 {
                let n = 1 + rng.below(3) as usize;
                tx.transport_mut().drop_next(n);
                dropped += n as u64;
            } else {
                let n = 1 + rng.below(3) as usize;
                tx.transport_mut().corrupt_next(n);
                corrupted += n as u64;
            }
        }
        tx.send(&payload_for(i)).expect("send");

        if (i + 1) % BATCH == 0 {
            for p in rx.poll().expect("poll") {
                delivered.push(p);
            }
        }
    }
    for p in rx.poll().expect("final poll") {
        delivered.push(p);
    }

    // No damaged frame may come out as data. Every delivered payload must be
    // exactly one the sender built, and indices must rise, since the loopback
    // keeps order.
    let mut last: Option<u32> = None;
    for p in &delivered {
        let index = index_of(p).expect("a delivered payload was damaged");
        if let Some(prev) = last {
            assert!(
                index > prev,
                "out of order or duplicate: {prev} then {index}"
            );
        }
        last = Some(index);
    }

    let stats = rx.stats();
    let sent = u64::from(MESSAGES);
    assert_eq!(tx.stats().sent, sent);

    // Every corrupted frame fails the CRC. Nothing else should.
    assert_eq!(
        stats.crc_errors, corrupted,
        "CRC failures should match injected corruption"
    );
    assert_eq!(stats.malformed, 0);

    // What arrived is what was sent, less what was dropped or corrupted.
    assert_eq!(delivered.len() as u64, sent - dropped - corrupted);
    assert_eq!(stats.received, delivered.len() as u64);

    // The receiver saw every missing frame as a sequence gap, so loss is exact.
    assert_eq!(
        stats.lost,
        dropped + corrupted,
        "sequence gaps should cover every missing frame"
    );

    // The run should really have exercised faults.
    assert!(dropped > 0 && corrupted > 0, "faults were not injected");
}
