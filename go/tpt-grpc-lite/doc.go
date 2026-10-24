// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

// Package grpclite is a small gRPC implementation over HTTP/2.
//
// It covers the parts of the gRPC wire protocol that tpt-hands uses: unary and
// streaming calls, status codes, trailers, deadlines (grpc-timeout), metadata,
// and TLS or mTLS. It has no compression and no load balancing or name
// resolution. After a connection drops, the HTTP/2 transport dials a new one on
// the next call.
//
// The wire format follows the gRPC over HTTP/2 specification. Messages are
// length-prefixed 5-byte frames, status is sent in the grpc-status trailer, and
// the path is /package.Service/Method.
//
// Not implemented: compression, retries, interceptors, reflection, and health
// checking.
package grpclite
