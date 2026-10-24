// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package grpclite

import (
	"context"
	"encoding/base64"
	"strings"
)

// MD is metadata: header-like key/value pairs. Keys are lowercase. A key that
// ends in "-bin" carries binary values, which are base64-encoded on the wire.
type MD map[string][]string

// NewMD returns an MD from alternating key and value strings. It panics on an
// odd count, which is a programming error.
func NewMD(kv ...string) MD {
	if len(kv)%2 != 0 {
		panic("grpclite: NewMD needs key/value pairs")
	}
	md := MD{}
	for i := 0; i < len(kv); i += 2 {
		md.Append(kv[i], kv[i+1])
	}
	return md
}

// Append adds a value under the lowercased key.
func (md MD) Append(key, value string) {
	k := strings.ToLower(key)
	md[k] = append(md[k], value)
}

// Get returns the values for key, or nil.
func (md MD) Get(key string) []string {
	return md[strings.ToLower(key)]
}

// Copy returns a copy of md that shares no slices with it.
func (md MD) Copy() MD {
	out := make(MD, len(md))
	for k, v := range md {
		out[k] = append([]string(nil), v...)
	}
	return out
}

type mdKey int

const (
	outgoingKey mdKey = iota
	incomingKey
)

// NewOutgoingContext attaches metadata to send with a client call.
func NewOutgoingContext(ctx context.Context, md MD) context.Context {
	return context.WithValue(ctx, outgoingKey, md.Copy())
}

// FromOutgoingContext returns the metadata set for a client call.
func FromOutgoingContext(ctx context.Context) (MD, bool) {
	md, ok := ctx.Value(outgoingKey).(MD)
	return md, ok
}

// FromIncomingContext returns the metadata a client sent with the call. Server
// handlers read it with this.
func FromIncomingContext(ctx context.Context) (MD, bool) {
	md, ok := ctx.Value(incomingKey).(MD)
	return md, ok
}

func withIncoming(ctx context.Context, md MD) context.Context {
	return context.WithValue(ctx, incomingKey, md)
}

// reservedHeader reports whether a header is part of the gRPC protocol or the
// HTTP layer, and so is not user metadata.
func reservedHeader(key string) bool {
	if strings.HasPrefix(key, "grpc-") || strings.HasPrefix(key, ":") {
		return true
	}
	switch key {
	case "content-type", "te", "user-agent", "host", "content-length",
		"trailer", "accept-encoding", "content-encoding":
		return true
	}
	return false
}

// encodeValue returns the wire form of a metadata value.
func encodeValue(key, value string) string {
	if strings.HasSuffix(key, "-bin") {
		return base64.StdEncoding.EncodeToString([]byte(value))
	}
	return value
}

// decodeValue reverses encodeValue. It accepts padded and unpadded base64.
func decodeValue(key, value string) (string, bool) {
	if !strings.HasSuffix(key, "-bin") {
		return value, true
	}
	if b, err := base64.StdEncoding.DecodeString(value); err == nil {
		return string(b), true
	}
	if b, err := base64.RawStdEncoding.DecodeString(value); err == nil {
		return string(b), true
	}
	return "", false
}
