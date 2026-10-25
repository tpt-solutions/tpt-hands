// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use ed25519_dalek::SigningKey;
use tpt_hands_handshake::{Hello, NONCE_LEN, Reason, Relay, Verifier, respond};

fn key() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

fn hello(counter: u64, nonce: u8) -> Hello {
    Hello {
        tool_id: "hand-0".into(),
        nonce: [nonce; NONCE_LEN],
        counter,
    }
}

// Two handshakes that both start before either completes. Both carry counter 6.
// Once the first closes the relay, the second must not close it, because its
// counter is no longer strictly newer.
#[test]
fn second_pending_handshake_with_a_used_counter_is_refused() {
    let mut v = Verifier::new(1_000_000);
    v.enroll("hand-0", key().verifying_key().to_bytes())
        .unwrap();

    let (ch_a, pending_a) = v.on_hello(0, hello(6, 1), [2u8; NONCE_LEN]).unwrap();
    let (ch_b, pending_b) = v.on_hello(0, hello(6, 3), [4u8; NONCE_LEN]).unwrap();

    let resp_a = respond(&key(), &hello(6, 1), &ch_a);
    assert_eq!(v.on_response(10, pending_a, &resp_a), Relay::Closed);

    let resp_b = respond(&key(), &hello(6, 3), &ch_b);
    assert_eq!(
        v.on_response(20, pending_b, &resp_b),
        Relay::Open(Reason::ReplayedCounter)
    );
}
