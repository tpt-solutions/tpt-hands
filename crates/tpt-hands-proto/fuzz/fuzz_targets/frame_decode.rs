// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_hands_proto::frame;

// Any input must decode or fail cleanly, without panicking. A frame that
// decodes must encode back to the same bytes.
fuzz_target!(|data: &[u8]| {
    if let Ok((decoded, used)) = frame::decode(data) {
        assert!(used <= data.len());
        let mut wire = Vec::new();
        frame::encode(decoded.seq, decoded.payload, &mut wire)
            .expect("a decoded frame must encode");
        assert_eq!(&data[..used], &wire[..]);
    }
});
