// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! tpt-edge firmware library (`no_std`).
//!
//! Reflex loops, pathway monitoring and the power handshake are added in
//! Phases 1 and 3 (see `todo.md`). Target-specific bring-up lives in
//! separate binaries built for the MCU target.

#![no_std]
#![forbid(unsafe_code)]

use tpt_hands_proto::frame::{self, Frame, FrameError};

/// Parses one frame received from the host, without copying the payload.
///
/// The firmware calls this from its receive path. A damaged frame returns an
/// error, and the caller drops it.
///
/// ```
/// use tpt_hands_edge::parse_frame;
///
/// assert!(parse_frame(&[]).is_err());
/// ```
pub fn parse_frame(buf: &[u8]) -> Result<Frame<'_>, FrameError> {
    frame::decode(buf).map(|(frame, _used)| frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_buffer_is_truncated() {
        assert_eq!(parse_frame(&[]), Err(FrameError::Truncated));
    }

    #[test]
    fn unknown_version_is_rejected() {
        assert_eq!(
            parse_frame(&[2, 0, 0, 0, 0, 0, 0]),
            Err(FrameError::BadVersion(2))
        );
    }

    #[test]
    fn corrupt_crc_is_rejected() {
        // Version 1, seq 0, empty payload, then a CRC that cannot match.
        let buf = [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(matches!(parse_frame(&buf), Err(FrameError::BadCrc { .. })));
    }
}
