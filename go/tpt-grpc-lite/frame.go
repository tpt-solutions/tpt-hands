// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package grpclite

import (
	"encoding/binary"
	"errors"
	"io"
	"math"
)

// DefaultMaxMessageSize is the largest message either side accepts unless a
// limit is set. It matches the gRPC default of 4 MiB.
const DefaultMaxMessageSize = 4 << 20

// frameHeaderLen is the size of the prefix on each message: one compressed
// flag byte and a 4-byte big-endian length.
const frameHeaderLen = 5

// writeFrame writes one uncompressed message frame.
func writeFrame(w io.Writer, payload []byte) error {
	if uint64(len(payload)) > math.MaxUint32 {
		return Errorf(ResourceExhausted, "message too large to frame: %d bytes", len(payload))
	}
	var hdr [frameHeaderLen]byte
	binary.BigEndian.PutUint32(hdr[1:], uint32(len(payload)))
	if _, err := w.Write(hdr[:]); err != nil {
		return err
	}
	_, err := w.Write(payload)
	return err
}

// readFrame reads one message frame. It returns io.EOF only when the stream
// ends cleanly between frames. A stream that ends inside a frame is Internal.
func readFrame(r io.Reader, maxSize int) ([]byte, error) {
	var hdr [frameHeaderLen]byte
	if _, err := io.ReadFull(r, hdr[:]); err != nil {
		if errors.Is(err, io.EOF) {
			return nil, io.EOF
		}
		return nil, Errorf(Internal, "truncated frame header")
	}
	if hdr[0] != 0 {
		return nil, Errorf(Unimplemented, "compressed messages are not supported")
	}
	n := binary.BigEndian.Uint32(hdr[1:])
	if uint64(n) > uint64(maxSize) {
		return nil, Errorf(ResourceExhausted, "message of %d bytes exceeds limit of %d", n, maxSize)
	}
	buf := make([]byte, n)
	if _, err := io.ReadFull(r, buf); err != nil {
		return nil, Errorf(Internal, "truncated message body")
	}
	return buf, nil
}
