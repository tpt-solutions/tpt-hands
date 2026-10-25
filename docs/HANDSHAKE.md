<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 TPT Solutions -->

# Power handshake protocol

This specifies the protocol as implemented in `crates/tpt-hands-handshake`. It
is the input to the formal model (todo.md, Phase 3). DESIGN.md section 7 gives the
intent. Where this file and the code differ, the code is the reference and this
file needs fixing.

**Status: reference implementation, not audited, not formally checked.** See
[THREAT_MODEL.md](THREAT_MODEL.md).

## Roles

- **Device**: the end-effector. Holds an Ed25519 private key. Never sends it.
- **Source**: the power side. Holds the enrolled public key for each `tool_id`,
  the last accepted counter for each device, and the relay.

## Messages

| Message | Sender | Fields |
| --- | --- | --- |
| `Hello` | device | `tool_id`, `nonce` (16 bytes), `counter` (u64) |
| `Challenge` | source | `nonce` (16 bytes), chosen by the caller |
| `Response` | device | `public_key` (32), `signature` (64), `counter` (u64) |

The source's answer is `Relay::Closed` or `Relay::Open(reason)`.

## Signed message

The device signs this byte string, and nothing else:

```
DOMAIN                       "tpt-hands/handshake/v1"  (22 bytes)
u16 LE len(tool_id)
tool_id                      UTF-8
hello.nonce                  16 bytes
challenge.nonce              16 bytes
u64 LE counter
```

The domain tag stops a signature from being valid in another protocol. Both
nonces stop a signature from being reused in another session. The counter stops
a response being reused for another request.

## Sequence

1. **Hello.** The device sends `Hello`. The source calls `on_hello(now, hello,
   challenge_nonce)`. It refuses the hello (no challenge, no pending state) if:
   - the `tool_id` is not enrolled (`UnknownTool`),
   - the hello nonce or the challenge nonce is all zero (`BadNonce`),
   - `counter <= last_counter[tool_id]` (`ReplayedCounter`).

   Otherwise it returns the `Challenge` and a `Pending` record that holds the
   hello, the challenge, and `now`.
2. **Response.** The device signs the challenge with `respond(key, hello,
   challenge)` and returns `Response`. The source calls `on_response(now,
   pending, response)`. Checks run in this order, and the first failure wins:

   | # | Check | Failure reason |
   | - | ----- | -------------- |
   | 1 | `now - started <= timeout` | `Timeout` |
   | 2 | `response.counter == hello.counter` | `OutOfOrder` |
   | 3 | `response.counter > last_counter[tool_id]` | `ReplayedCounter` |
   | 4 | `public_key` parses as an Ed25519 point | `MalformedKey` |
   | 5 | `public_key` equals the enrolled key for `tool_id` | `WrongKey` |
   | 6 | `signature` verifies over the signed message | `BadSignature` |

   If every check passes, the source records `last_counter[tool_id] =
   response.counter` and closes the relay.

## Failure default

Any failure leaves the relay **open**. The relay closes only on a full pass of
checks 1 to 6. A refused handshake does not advance the counter.

## Counter rules

- The counter is per device and strictly increases on every accepted handshake.
- It is checked **twice**: when the hello arrives, and when the response arrives.
  The second check matters when two handshakes overlap. Before the fix recorded in
  THREAT_MODEL.md (T9), the second response could close the relay with a counter
  that had already been used.
- The device chooses the counter. The source does not check that it is fresh
  beyond the rules above.

## Nonces

- The source takes the challenge nonce from its caller, and the caller must draw
  it from a CSPRNG. The crate has no RNG, so this is not enforced by types.
- The hello nonce is chosen by the device. It is only checked for being non-zero.

## Enrolment

`Verifier::enroll(tool_id, public_key)` stores the key. How keys are provisioned,
stored, rotated and revoked is not designed yet. That is an open item.

## What is not in the spec yet

- Persistent counter storage across power loss. The counter is in memory.
- Key rotation and revocation. Enrolment cannot be undone.
- The relay timing requirement: link loss opens the relay in under 1 ms. That is
  a property of the hardware driver, not of this protocol.
- A transport binding. The messages are not yet encoded on the wire.
