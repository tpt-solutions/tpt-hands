// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Stable-toolchain stand-in for the `frame_decode` cargo-fuzz target. It runs
//! the same property on fixed pseudo-random inputs, so CI checks frame parsing
//! without nightly.

use tpt_hands_proto::frame;

/// xorshift64, so the inputs are the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn check(data: &[u8]) {
    if let Ok((decoded, used)) = frame::decode(data) {
        assert!(used <= data.len());
        let mut wire = Vec::new();
        frame::encode(decoded.seq, decoded.payload, &mut wire).expect("decoded frame encodes");
        assert_eq!(&data[..used], &wire[..]);
    }
}

#[test]
fn random_bytes_never_panic_and_valid_frames_round_trip() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..200_000 {
        let len = (rng.next() % 64) as usize;
        let data: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
        check(&data);
    }
}

#[test]
fn mutated_valid_frames_are_handled() {
    // Start from a real frame and flip one byte at a time. Every result must
    // either fail, or decode back to the same bytes.
    let mut base = Vec::new();
    frame::encode(7, b"hello, tendon", &mut base).unwrap();
    for i in 0..base.len() {
        for bit in 0..8 {
            let mut m = base.clone();
            m[i] ^= 1 << bit;
            check(&m);
        }
    }
    check(&base);
}
