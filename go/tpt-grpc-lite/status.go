// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package grpclite

import (
	"context"
	"errors"
	"fmt"
	"strconv"
	"strings"
)

// Code is a gRPC status code. Values match the gRPC specification.
type Code uint32

// Status codes.
const (
	OK                 Code = 0
	Canceled           Code = 1
	Unknown            Code = 2
	InvalidArgument    Code = 3
	DeadlineExceeded   Code = 4
	NotFound           Code = 5
	AlreadyExists      Code = 6
	PermissionDenied   Code = 7
	ResourceExhausted  Code = 8
	FailedPrecondition Code = 9
	Aborted            Code = 10
	OutOfRange         Code = 11
	Unimplemented      Code = 12
	Internal           Code = 13
	Unavailable        Code = 14
	DataLoss           Code = 15
	Unauthenticated    Code = 16
)

// Error is a status error. A handler returns one to set the code the client
// sees.
type Error struct {
	Code    Code
	Message string
}

func (e *Error) Error() string {
	if e.Message == "" {
		return fmt.Sprintf("grpc: code %d", e.Code)
	}
	return fmt.Sprintf("grpc: code %d: %s", e.Code, e.Message)
}

// Errorf returns a status error with a formatted message.
func Errorf(c Code, format string, args ...any) error {
	return &Error{Code: c, Message: fmt.Sprintf(format, args...)}
}

// FromError returns the status code and message for err. A nil error is OK.
// Context errors map to Canceled and DeadlineExceeded. Any other error is
// Unknown, and its text is not sent to the client, so internal details do not
// leak.
func FromError(err error) (Code, string) {
	if err == nil {
		return OK, ""
	}
	var se *Error
	if errors.As(err, &se) {
		return se.Code, se.Message
	}
	switch {
	case errors.Is(err, context.DeadlineExceeded):
		return DeadlineExceeded, "deadline exceeded"
	case errors.Is(err, context.Canceled):
		return Canceled, "context canceled"
	}
	return Unknown, "unknown error"
}

// encodeMessage percent-encodes a status message for the grpc-message trailer.
// Bytes outside printable ASCII, and '%', are escaped.
func encodeMessage(msg string) string {
	var b strings.Builder
	for i := 0; i < len(msg); i++ {
		c := msg[i]
		if c < 0x20 || c > 0x7e || c == '%' {
			fmt.Fprintf(&b, "%%%02X", c)
		} else {
			b.WriteByte(c)
		}
	}
	return b.String()
}

// decodeMessage reverses encodeMessage. A malformed escape is kept as is.
func decodeMessage(s string) string {
	var b strings.Builder
	for i := 0; i < len(s); i++ {
		if s[i] == '%' && i+2 < len(s) {
			if v, err := strconv.ParseUint(s[i+1:i+3], 16, 8); err == nil {
				b.WriteByte(byte(v))
				i += 2
				continue
			}
		}
		b.WriteByte(s[i])
	}
	return b.String()
}
