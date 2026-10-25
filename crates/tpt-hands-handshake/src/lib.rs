// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Smart power handshake (DESIGN.md section 7).
//!
//! Order of events: the data pins mate first, then the device sends a hello and
//! the source returns a challenge. The device signs and returns a response. The
//! source verifies it and closes the relay. Any failure leaves the relay open.
//!
//! What this crate checks:
//! - the device key matches the key enrolled for its `tool_id`
//! - the signature covers the device id, both nonces and the counter, under a
//!   domain tag, so it cannot be reused in another context
//! - the counter must strictly increase per device, which blocks replays
//! - a handshake that takes longer than the timeout is rejected
//!
//! The verifier takes the challenge nonce from its caller. That keeps the crate
//! free of a random-number source. The caller must draw it from a CSPRNG.
//!
//! **Status: not audited.** The spec (todo.md, Phase 3) requires a formal model
//! and a security review before this protects real power. This crate is the
//! reference implementation those steps will check.

use std::collections::BTreeMap;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier as _, VerifyingKey};

/// Domain tag for every signed message. Stops a signature made here from being
/// valid in another protocol.
pub const DOMAIN: &[u8] = b"tpt-hands/handshake/v1";

/// Length of every nonce.
pub const NONCE_LEN: usize = 16;

/// Device to source: "I am here, and this is my nonce and counter."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hello {
    pub tool_id: String,
    pub nonce: [u8; NONCE_LEN],
    pub counter: u64,
}

/// Source to device: a fresh nonce for the device to sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Challenge {
    pub nonce: [u8; NONCE_LEN],
}

/// Device to source: the key, the signature, and the counter it signed under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub public_key: [u8; 32],
    pub signature: [u8; 64],
    pub counter: u64,
}

/// Why a handshake was refused. The relay stays open for every one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// No key is enrolled for this tool id.
    UnknownTool,
    /// A nonce is all zeros.
    BadNonce,
    /// The counter is not higher than the last accepted one.
    ReplayedCounter,
    /// The response counter does not match the hello.
    OutOfOrder,
    /// The public key bytes are not a valid Ed25519 point.
    MalformedKey,
    /// The key is valid, but it is not the one enrolled for this tool.
    WrongKey,
    /// The signature does not verify.
    BadSignature,
    /// The handshake took longer than the timeout.
    Timeout,
}

/// The relay decision for a finished handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relay {
    /// Verified. The relay may close.
    Closed,
    /// Not verified, for the stated reason. The relay stays open.
    Open(Reason),
}

/// The bytes the device signs. Domain tag, then a length-prefixed tool id,
/// then the hello nonce, the challenge nonce and the counter (little-endian).
pub fn signed_message(
    tool_id: &str,
    hello_nonce: &[u8; NONCE_LEN],
    challenge_nonce: &[u8; NONCE_LEN],
    counter: u64,
) -> Vec<u8> {
    let id = tool_id.as_bytes();
    let mut msg = Vec::with_capacity(DOMAIN.len() + 2 + id.len() + 2 * NONCE_LEN + 8);
    msg.extend_from_slice(DOMAIN);
    msg.extend_from_slice(&(id.len() as u16).to_le_bytes());
    msg.extend_from_slice(id);
    msg.extend_from_slice(hello_nonce);
    msg.extend_from_slice(challenge_nonce);
    msg.extend_from_slice(&counter.to_le_bytes());
    msg
}

/// Device side: signs the challenge for the hello that started this handshake.
///
/// ```
/// use tpt_hands_handshake::{respond, Challenge, Hello};
/// use ed25519_dalek::SigningKey;
///
/// let key = SigningKey::from_bytes(&[7u8; 32]);
/// let hello = Hello { tool_id: "hand-0".into(), nonce: [1; 16], counter: 1 };
/// let resp = respond(&key, &hello, &Challenge { nonce: [2; 16] });
/// assert_eq!(resp.counter, 1);
/// ```
pub fn respond(key: &SigningKey, hello: &Hello, challenge: &Challenge) -> Response {
    let msg = signed_message(
        &hello.tool_id,
        &hello.nonce,
        &challenge.nonce,
        hello.counter,
    );
    let signature: Signature = key.sign(&msg);
    Response {
        public_key: key.verifying_key().to_bytes(),
        signature: signature.to_bytes(),
        counter: hello.counter,
    }
}

/// A handshake that has sent its challenge and is waiting for the response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    hello: Hello,
    challenge: Challenge,
    started_us: u64,
}

impl Pending {
    /// Tool this handshake belongs to.
    pub fn tool_id(&self) -> &str {
        &self.hello.tool_id
    }
}

/// Source side. Keeps the enrolled keys and the last accepted counter per tool.
#[derive(Debug, Clone)]
pub struct Verifier {
    keys: BTreeMap<String, VerifyingKey>,
    last_counter: BTreeMap<String, u64>,
    timeout_us: u64,
}

impl Verifier {
    /// Creates a verifier. A handshake must finish within `timeout_us`.
    pub fn new(timeout_us: u64) -> Self {
        Self {
            keys: BTreeMap::new(),
            last_counter: BTreeMap::new(),
            timeout_us,
        }
    }

    /// Enrolls a device key. Enrollment is out of scope here: the caller decides
    /// how keys reach the source. A malformed key is refused.
    pub fn enroll(&mut self, tool_id: &str, public_key: [u8; 32]) -> Result<(), Reason> {
        let key = VerifyingKey::from_bytes(&public_key).map_err(|_| Reason::MalformedKey)?;
        self.keys.insert(tool_id.to_string(), key);
        Ok(())
    }

    /// Handles a hello. `challenge_nonce` must be fresh random bytes from a
    /// CSPRNG. Returns the challenge to send, and the pending handshake.
    pub fn on_hello(
        &self,
        now_us: u64,
        hello: Hello,
        challenge_nonce: [u8; NONCE_LEN],
    ) -> Result<(Challenge, Pending), Reason> {
        if !self.keys.contains_key(&hello.tool_id) {
            return Err(Reason::UnknownTool);
        }
        if hello.nonce == [0u8; NONCE_LEN] || challenge_nonce == [0u8; NONCE_LEN] {
            return Err(Reason::BadNonce);
        }
        if let Some(&last) = self.last_counter.get(&hello.tool_id) {
            if hello.counter <= last {
                return Err(Reason::ReplayedCounter);
            }
        }
        let challenge = Challenge {
            nonce: challenge_nonce,
        };
        let pending = Pending {
            hello,
            challenge,
            started_us: now_us,
        };
        Ok((challenge, pending))
    }

    /// Handles the response. On success, the counter is recorded and the relay
    /// may close. On any failure, the reason is returned and the counter is not
    /// recorded.
    pub fn on_response(&mut self, now_us: u64, pending: Pending, response: &Response) -> Relay {
        match self.verify(now_us, &pending, response) {
            Ok(()) => {
                self.last_counter
                    .insert(pending.hello.tool_id.clone(), response.counter);
                Relay::Closed
            }
            Err(reason) => Relay::Open(reason),
        }
    }

    fn verify(&self, now_us: u64, pending: &Pending, response: &Response) -> Result<(), Reason> {
        if now_us.saturating_sub(pending.started_us) > self.timeout_us {
            return Err(Reason::Timeout);
        }
        if response.counter != pending.hello.counter {
            return Err(Reason::OutOfOrder);
        }
        // Checked at response time as well as at hello time. Another handshake
        // may have completed while this one was pending, and it may have used the
        // same counter.
        if let Some(&last) = self.last_counter.get(&pending.hello.tool_id) {
            if response.counter <= last {
                return Err(Reason::ReplayedCounter);
            }
        }
        let presented =
            VerifyingKey::from_bytes(&response.public_key).map_err(|_| Reason::MalformedKey)?;
        let enrolled = self
            .keys
            .get(&pending.hello.tool_id)
            .ok_or(Reason::UnknownTool)?;
        if presented != *enrolled {
            return Err(Reason::WrongKey);
        }
        let msg = signed_message(
            &pending.hello.tool_id,
            &pending.hello.nonce,
            &pending.challenge.nonce,
            response.counter,
        );
        let signature = Signature::from_bytes(&response.signature);
        presented
            .verify(&msg, &signature)
            .map_err(|_| Reason::BadSignature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn hello(counter: u64) -> Hello {
        Hello {
            tool_id: "hand-0".into(),
            nonce: [1u8; NONCE_LEN],
            counter,
        }
    }

    fn enrolled() -> Verifier {
        let mut v = Verifier::new(1_000_000);
        v.enroll("hand-0", device_key().verifying_key().to_bytes())
            .unwrap();
        v
    }

    /// Runs a full handshake and returns the relay decision.
    fn run(v: &mut Verifier, key: &SigningKey, counter: u64, now: u64) -> Relay {
        let (challenge, pending) = v
            .on_hello(now, hello(counter), [2u8; NONCE_LEN])
            .expect("hello accepted");
        let response = respond(key, &hello(counter), &challenge);
        v.on_response(now + 10, pending, &response)
    }

    #[test]
    fn valid_handshake_closes_the_relay() {
        let mut v = enrolled();
        assert_eq!(run(&mut v, &device_key(), 1, 0), Relay::Closed);
    }

    #[test]
    fn replayed_counter_is_refused() {
        let mut v = enrolled();
        assert_eq!(run(&mut v, &device_key(), 5, 0), Relay::Closed);
        // The same counter again is a replay, refused at the hello.
        assert_eq!(
            v.on_hello(0, hello(5), [2u8; NONCE_LEN]),
            Err(Reason::ReplayedCounter)
        );
        // A lower counter is refused too.
        assert_eq!(
            v.on_hello(0, hello(4), [2u8; NONCE_LEN]),
            Err(Reason::ReplayedCounter)
        );
        // A higher counter is accepted.
        assert_eq!(run(&mut v, &device_key(), 6, 0), Relay::Closed);
    }

    #[test]
    fn unknown_tool_is_refused_at_hello() {
        let v = Verifier::new(1_000);
        let mut h = hello(1);
        h.tool_id = "stranger".into();
        assert_eq!(v.on_hello(0, h, [2u8; NONCE_LEN]), Err(Reason::UnknownTool));
    }

    #[test]
    fn wrong_key_is_refused() {
        let mut v = enrolled();
        let impostor = SigningKey::from_bytes(&[9u8; 32]);
        assert_eq!(run(&mut v, &impostor, 1, 0), Relay::Open(Reason::WrongKey));
    }

    #[test]
    fn signature_over_a_different_challenge_is_refused() {
        let mut v = enrolled();
        let (_challenge, pending) = v.on_hello(0, hello(1), [2u8; NONCE_LEN]).unwrap();
        // The device signed some other challenge.
        let other = Challenge {
            nonce: [3u8; NONCE_LEN],
        };
        let response = respond(&device_key(), &hello(1), &other);
        assert_eq!(
            v.on_response(10, pending, &response),
            Relay::Open(Reason::BadSignature)
        );
    }

    #[test]
    fn tampered_counter_is_refused() {
        let mut v = enrolled();
        let (challenge, pending) = v.on_hello(0, hello(1), [2u8; NONCE_LEN]).unwrap();
        let mut response = respond(&device_key(), &hello(1), &challenge);
        // Change the counter after signing. The response then fails the match
        // check, or the signature check if the counter were to match.
        response.counter = 2;
        assert_eq!(
            v.on_response(10, pending, &response),
            Relay::Open(Reason::OutOfOrder)
        );
    }

    #[test]
    fn signature_from_another_tool_context_is_refused() {
        // A signature made for another tool id must not verify for this one.
        let mut v = enrolled();
        let (challenge, pending) = v.on_hello(0, hello(1), [2u8; NONCE_LEN]).unwrap();
        let mut other = hello(1);
        other.tool_id = "hand-1".into();
        let response = respond(&device_key(), &other, &challenge);
        assert_eq!(
            v.on_response(10, pending, &response),
            Relay::Open(Reason::BadSignature)
        );
    }

    #[test]
    fn slow_handshake_times_out() {
        let mut v = enrolled();
        let (challenge, pending) = v.on_hello(0, hello(1), [2u8; NONCE_LEN]).unwrap();
        let response = respond(&device_key(), &hello(1), &challenge);
        assert_eq!(
            v.on_response(2_000_000, pending, &response),
            Relay::Open(Reason::Timeout)
        );
    }

    #[test]
    fn zero_nonces_are_refused() {
        let v = enrolled();
        let zero_hello = Hello {
            nonce: [0u8; NONCE_LEN],
            ..hello(1)
        };
        assert_eq!(
            v.on_hello(0, zero_hello, [2u8; NONCE_LEN]),
            Err(Reason::BadNonce)
        );
        assert_eq!(
            v.on_hello(0, hello(1), [0u8; NONCE_LEN]),
            Err(Reason::BadNonce)
        );
    }

    #[test]
    fn a_refused_handshake_does_not_advance_the_counter() {
        let mut v = enrolled();
        let impostor = SigningKey::from_bytes(&[9u8; 32]);
        assert_eq!(run(&mut v, &impostor, 3, 0), Relay::Open(Reason::WrongKey));
        // Counter 3 was never accepted, so the real device can still use it.
        assert_eq!(run(&mut v, &device_key(), 3, 0), Relay::Closed);
    }

    #[test]
    fn enrolling_a_malformed_key_is_refused() {
        let mut v = Verifier::new(1_000);
        // Found by search: this encoding decompresses to no valid Ed25519 point.
        let mut bad = [0u8; 32];
        bad[0] = 0x02;
        assert_eq!(v.enroll("hand-0", bad), Err(Reason::MalformedKey));
    }

    #[test]
    fn signed_message_is_domain_separated_and_binds_every_field() {
        let base = signed_message("hand-0", &[1; 16], &[2; 16], 1);
        assert!(base.starts_with(DOMAIN));
        assert_ne!(base, signed_message("hand-1", &[1; 16], &[2; 16], 1));
        assert_ne!(base, signed_message("hand-0", &[9; 16], &[2; 16], 1));
        assert_ne!(base, signed_message("hand-0", &[1; 16], &[9; 16], 1));
        assert_ne!(base, signed_message("hand-0", &[1; 16], &[2; 16], 2));
    }
}
