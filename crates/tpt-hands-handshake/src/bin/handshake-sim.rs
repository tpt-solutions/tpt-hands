// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Fault-injection simulator for the power handshake (todo.md, Phase 3).
//!
//! Runs each fault case, checks the relay decision against the expected one,
//! and prints a table. Exits non-zero if any case differs from its expectation.
//! Run with `cargo run -p tpt-hands-handshake --bin handshake-sim`.

use ed25519_dalek::SigningKey;
use tpt_hands_handshake::{Challenge, Hello, NONCE_LEN, Reason, Relay, Verifier, respond};

const TIMEOUT_US: u64 = 100_000;

fn device() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

fn hello(counter: u64) -> Hello {
    Hello {
        tool_id: "hand-0".into(),
        nonce: [1u8; NONCE_LEN],
        counter,
    }
}

fn fresh_verifier() -> Verifier {
    let mut v = Verifier::new(TIMEOUT_US);
    v.enroll("hand-0", device().verifying_key().to_bytes())
        .expect("enroll");
    v
}

/// One fault case: a name, the expected relay decision, and the run.
struct Case {
    name: &'static str,
    expected: Relay,
    run: fn() -> Relay,
}

fn clean_handshake() -> Relay {
    let mut v = fresh_verifier();
    let (challenge, pending) = v.on_hello(0, hello(1), [2u8; NONCE_LEN]).unwrap();
    let resp = respond(&device(), &hello(1), &challenge);
    v.on_response(10, pending, &resp)
}

fn bad_signature() -> Relay {
    // The device signs a different challenge than the one it was sent.
    let mut v = fresh_verifier();
    let (_c, pending) = v.on_hello(0, hello(1), [2u8; NONCE_LEN]).unwrap();
    let other = Challenge {
        nonce: [9u8; NONCE_LEN],
    };
    let resp = respond(&device(), &hello(1), &other);
    v.on_response(10, pending, &resp)
}

fn replay() -> Relay {
    // A captured response is sent again after the handshake succeeded.
    let mut v = fresh_verifier();
    let (challenge, pending) = v.on_hello(0, hello(1), [2u8; NONCE_LEN]).unwrap();
    let resp = respond(&device(), &hello(1), &challenge);
    assert_eq!(v.on_response(10, pending, &resp), Relay::Closed);
    // The attacker replays the hello with the same counter.
    match v.on_hello(20, hello(1), [3u8; NONCE_LEN]) {
        Err(reason) => Relay::Open(reason),
        Ok(_) => Relay::Closed,
    }
}

fn cable_pulled_then_reconnect() -> Relay {
    // The hello and challenge went out, then the link dropped before any response.
    // The pending handshake is discarded. The device reconnects and sends the
    // same counter again. Nothing was accepted, so the counter is still free and
    // the new handshake succeeds.
    let mut v = fresh_verifier();
    let (_lost_challenge, lost_pending) = v.on_hello(0, hello(1), [2u8; NONCE_LEN]).unwrap();
    drop(lost_pending);
    let (challenge, pending) = v.on_hello(50, hello(1), [4u8; NONCE_LEN]).unwrap();
    let resp = respond(&device(), &hello(1), &challenge);
    v.on_response(60, pending, &resp)
}

fn late_response_clock_skew() -> Relay {
    // The response arrives after the timeout, for example because of clock skew.
    let mut v = fresh_verifier();
    let (challenge, pending) = v.on_hello(0, hello(1), [2u8; NONCE_LEN]).unwrap();
    let resp = respond(&device(), &hello(1), &challenge);
    v.on_response(TIMEOUT_US + 1, pending, &resp)
}

fn wrong_device_key() -> Relay {
    let mut v = fresh_verifier();
    let (challenge, pending) = v.on_hello(0, hello(1), [2u8; NONCE_LEN]).unwrap();
    let impostor = SigningKey::from_bytes(&[9u8; 32]);
    let resp = respond(&impostor, &hello(1), &challenge);
    v.on_response(10, pending, &resp)
}

fn unknown_device() -> Relay {
    let v = Verifier::new(TIMEOUT_US);
    match v.on_hello(0, hello(1), [2u8; NONCE_LEN]) {
        Err(reason) => Relay::Open(reason),
        Ok(_) => Relay::Closed,
    }
}

fn main() {
    let cases = [
        Case {
            name: "clean handshake",
            expected: Relay::Closed,
            run: clean_handshake,
        },
        Case {
            name: "bad signature",
            expected: Relay::Open(Reason::BadSignature),
            run: bad_signature,
        },
        Case {
            name: "replayed counter",
            expected: Relay::Open(Reason::ReplayedCounter),
            run: replay,
        },
        Case {
            name: "cable pulled, then reconnect",
            expected: Relay::Closed,
            run: cable_pulled_then_reconnect,
        },
        Case {
            name: "late response (clock skew)",
            expected: Relay::Open(Reason::Timeout),
            run: late_response_clock_skew,
        },
        Case {
            name: "wrong device key",
            expected: Relay::Open(Reason::WrongKey),
            run: wrong_device_key,
        },
        Case {
            name: "unknown device",
            expected: Relay::Open(Reason::UnknownTool),
            run: unknown_device,
        },
    ];

    println!("{:<30} {:<28} {:<28} result", "case", "expected", "actual");
    let mut failures = 0;
    for case in &cases {
        let actual = (case.run)();
        let ok = actual == case.expected;
        if !ok {
            failures += 1;
        }
        println!(
            "{:<30} {:<28} {:<28} {}",
            case.name,
            format!("{:?}", case.expected),
            format!("{:?}", actual),
            if ok { "PASS" } else { "FAIL" }
        );
    }
    if failures > 0 {
        eprintln!("{failures} case(s) did not match the expected relay decision");
        std::process::exit(1);
    }
}
