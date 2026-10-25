// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Bring-up walk-through for a smart socket. It enrols a device, runs the
//! handshake that closes the relay, and then shows the refusals the source gives
//! for a replay and for a device with the wrong key.
//!
//! This is software only. It is not the hardware bring-up, which is a todo.md
//! Phase 3 item, and it needs a relay and a real device.
//!
//! Run with `cargo run -p tpt-hands-handshake --example smart_socket_bringup`.

use ed25519_dalek::SigningKey;
use tpt_hands_handshake::{Hello, NONCE_LEN, Relay, Verifier, respond};

const TOOL: &str = "socket-0";

/// Runs one handshake from the device side, the way a socket would: hello, then
/// the source's challenge, then the signed response.
fn handshake(
    source: &mut Verifier,
    device: &SigningKey,
    counter: u64,
    now_us: u64,
    challenge_nonce: u8,
) -> Relay {
    let hello = Hello {
        tool_id: TOOL.to_string(),
        nonce: [0x11; NONCE_LEN],
        counter,
    };
    let (challenge, pending) =
        match source.on_hello(now_us, hello.clone(), [challenge_nonce; NONCE_LEN]) {
            Ok(v) => v,
            Err(reason) => return Relay::Open(reason),
        };
    let response = respond(device, &hello, &challenge);
    source.on_response(now_us + 1_000, pending, &response)
}

fn main() {
    // Step 1: the factory enrols the device's public key with the source.
    let device = SigningKey::from_bytes(&[0x42; 32]);
    let mut source = Verifier::new(100_000);
    source
        .enroll(TOOL, device.verifying_key().to_bytes())
        .expect("factory key is valid");
    println!("1. enrolled {TOOL}");

    // Step 2: the first handshake, with counter 1, closes the relay.
    let first = handshake(&mut source, &device, 1, 0, 0x21);
    println!("2. handshake, counter 1      -> {first:?}");
    assert_eq!(first, Relay::Closed);

    // Step 3: the next handshake must use a higher counter. Replaying counter 1
    // is refused, so an old response cannot be used again.
    let replay = handshake(&mut source, &device, 1, 2_000_000, 0x22);
    println!("3. replay of counter 1       -> {replay:?}");

    // Step 4: a later handshake with counter 2 succeeds.
    let second = handshake(&mut source, &device, 2, 4_000_000, 0x23);
    println!("4. handshake, counter 2      -> {second:?}");
    assert_eq!(second, Relay::Closed);

    // Step 5: a device with a different key cannot close the relay, even with a
    // valid counter.
    let impostor = SigningKey::from_bytes(&[0x99; 32]);
    let refused = handshake(&mut source, &impostor, 3, 6_000_000, 0x24);
    println!("5. impostor key, counter 3   -> {refused:?}");
    assert!(matches!(refused, Relay::Open(_)));

    println!("done. Hardware steps (relay driver, link-loss timing) are not covered here.");
}
