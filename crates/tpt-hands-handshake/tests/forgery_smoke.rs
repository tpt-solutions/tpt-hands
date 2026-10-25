// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Stable-toolchain stand-in for the `forged_response` cargo-fuzz target. Random
//! responses must never close the relay.

use ed25519_dalek::SigningKey;
use tpt_hands_handshake::{Hello, NONCE_LEN, Relay, Response, Verifier, respond};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn bytes<const N: usize>(&mut self) -> [u8; N] {
        let mut out = [0u8; N];
        for b in out.iter_mut() {
            *b = self.next() as u8;
        }
        out
    }
}

fn setup() -> (SigningKey, Verifier) {
    let key = SigningKey::from_bytes(&[7u8; 32]);
    let mut verifier = Verifier::new(1_000_000);
    verifier
        .enroll("tool-a", key.verifying_key().to_bytes())
        .unwrap();
    (key, verifier)
}

#[test]
fn random_responses_never_close_the_relay() {
    let (_key, mut verifier) = setup();
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    for _ in 0..20_000 {
        let hello = Hello {
            tool_id: "tool-a".to_string(),
            nonce: rng.bytes::<NONCE_LEN>(),
            counter: 1,
        };
        let Ok((_c, pending)) = verifier.on_hello(0, hello, rng.bytes::<NONCE_LEN>()) else {
            continue;
        };
        let response = Response {
            public_key: rng.bytes::<32>(),
            signature: {
                let a = rng.bytes::<32>();
                let b = rng.bytes::<32>();
                let mut s = [0u8; 64];
                s[..32].copy_from_slice(&a);
                s[32..].copy_from_slice(&b);
                s
            },
            counter: rng.next(),
        };
        assert_ne!(verifier.on_response(0, pending, &response), Relay::Closed);
    }
}

#[test]
fn genuine_response_still_closes_the_relay() {
    // Control: the same setup with a real signature must close, so the property
    // above is not passing only because nothing can ever close.
    let (key, mut verifier) = setup();
    let hello = Hello {
        tool_id: "tool-a".to_string(),
        nonce: [3u8; NONCE_LEN],
        counter: 1,
    };
    let (challenge, pending) = verifier
        .on_hello(0, hello.clone(), [4u8; NONCE_LEN])
        .unwrap();
    let response = respond(&key, &hello, &challenge);
    assert_eq!(verifier.on_response(0, pending, &response), Relay::Closed);
}
