// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

#![no_main]

use ed25519_dalek::SigningKey;
use libfuzzer_sys::fuzz_target;
use tpt_hands_handshake::{Hello, Relay, Response, Verifier, NONCE_LEN};

/// Input layout: public key (32), signature (64), response counter (8),
/// hello nonce (16), challenge nonce (16).
const LEN: usize = 32 + 64 + 8 + NONCE_LEN + NONCE_LEN;

// Random bytes must never close the relay. Closing needs a signature that
// verifies under the enrolled key, and the fuzzer does not hold that key.
fuzz_target!(|data: &[u8]| {
    if data.len() < LEN {
        return;
    }
    let key = SigningKey::from_bytes(&[7u8; 32]);
    let mut verifier = Verifier::new(1_000_000);
    verifier
        .enroll("tool-a", key.verifying_key().to_bytes())
        .expect("test key is valid");

    let mut hello_nonce = [0u8; NONCE_LEN];
    hello_nonce.copy_from_slice(&data[104..120]);
    let mut challenge_nonce = [0u8; NONCE_LEN];
    challenge_nonce.copy_from_slice(&data[120..136]);
    let hello = Hello {
        tool_id: "tool-a".to_string(),
        nonce: hello_nonce,
        counter: 1,
    };
    let Ok((_challenge, pending)) = verifier.on_hello(0, hello, challenge_nonce) else {
        return;
    };

    let mut public_key = [0u8; 32];
    public_key.copy_from_slice(&data[0..32]);
    let mut signature = [0u8; 64];
    signature.copy_from_slice(&data[32..96]);
    let counter = u64::from_le_bytes(data[96..104].try_into().unwrap());
    let response = Response {
        public_key,
        signature,
        counter,
    };
    assert_ne!(verifier.on_response(0, pending, &response), Relay::Closed);
});
