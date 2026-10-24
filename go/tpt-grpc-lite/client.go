// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package grpclite

import (
	"context"
	"crypto/tls"
	"errors"
	"io"
	"net"
	"net/http"
	"strconv"
	"strings"
	"sync"
	"time"

	"golang.org/x/net/http2"
)

// ClientConn is a client for one target. It is safe for concurrent use. The
// HTTP/2 transport opens connections as needed and reuses them.
type ClientConn struct {
	scheme  string
	target  string
	hc      *http.Client
	t       *http2.Transport
	codec   Codec
	maxRecv int
}

// DialOption configures a ClientConn.
type DialOption func(*dialOptions)

type dialOptions struct {
	tls           *tls.Config
	codec         Codec
	maxRecv       int
	keepaliveIdle time.Duration
	keepaliveWait time.Duration
}

// WithTLS makes the connection use TLS. Set RootCAs in cfg to trust the
// server, and Certificates to present a client certificate (mTLS).
func WithTLS(cfg *tls.Config) DialOption {
	return func(o *dialOptions) { o.tls = cfg }
}

// WithCodec sets the message codec. The default is ProtoCodec.
func WithCodec(c Codec) DialOption {
	return func(o *dialOptions) { o.codec = c }
}

// WithMaxReceiveMessageSize sets the largest response message accepted.
func WithMaxReceiveMessageSize(n int) DialOption {
	return func(o *dialOptions) { o.maxRecv = n }
}

// WithKeepalive sends an HTTP/2 PING after the connection has been idle for
// idle. It closes the connection if no reply arrives within wait. Without it, a
// dead peer is only noticed on the next call.
func WithKeepalive(idle, wait time.Duration) DialOption {
	return func(o *dialOptions) {
		o.keepaliveIdle = idle
		o.keepaliveWait = wait
	}
}

// Dial prepares a connection to target, given as "host:port". It does not
// connect yet. The first call does.
func Dial(target string, opts ...DialOption) *ClientConn {
	o := dialOptions{codec: ProtoCodec{}, maxRecv: DefaultMaxMessageSize}
	for _, opt := range opts {
		opt(&o)
	}

	scheme := "http"
	if o.tls != nil {
		scheme = "https"
	}
	t := &http2.Transport{
		AllowHTTP:       o.tls == nil,
		TLSClientConfig: o.tls,
		ReadIdleTimeout: o.keepaliveIdle,
		PingTimeout:     o.keepaliveWait,
		DialTLSContext: func(ctx context.Context, network, addr string, _ *tls.Config) (net.Conn, error) {
			var d net.Dialer
			conn, err := d.DialContext(ctx, network, addr)
			if err != nil || o.tls == nil {
				return conn, err
			}
			cfg := o.tls.Clone()
			// Without ALPN the server falls back to HTTP/1.1, which breaks the call.
			if len(cfg.NextProtos) == 0 {
				cfg.NextProtos = []string{"h2"}
			}
			if cfg.ServerName == "" {
				host, _, _ := net.SplitHostPort(addr)
				cfg.ServerName = host
			}
			tc := tls.Client(conn, cfg)
			if err := tc.HandshakeContext(ctx); err != nil {
				conn.Close()
				return nil, err
			}
			return tc, nil
		},
	}
	return &ClientConn{
		scheme:  scheme,
		target:  target,
		hc:      &http.Client{Transport: t},
		t:       t,
		codec:   o.codec,
		maxRecv: o.maxRecv,
	}
}

// Close closes idle connections. Calls in progress are not affected.
func (cc *ClientConn) Close() error {
	cc.t.CloseIdleConnections()
	return nil
}

// ClientStream is one call from the client's side.
type ClientStream interface {
	// Header waits for the response headers and returns them as metadata.
	Header() (MD, error)
	// Trailer returns the trailing metadata. Read it after RecvMsg returns io.EOF.
	Trailer() MD
	// SendMsg sends one request message.
	SendMsg(m any) error
	// CloseSend tells the server no more requests are coming.
	CloseSend() error
	// RecvMsg reads one response message. It returns io.EOF when the call ends
	// with OK, and a *Error for any other status.
	RecvMsg(m any) error
}

type clientStream struct {
	cc      *ClientConn
	ctx     context.Context
	cancel  context.CancelFunc
	pw      *io.PipeWriter
	respc   chan respResult
	closed  sync.Once
	recvd   bool
	resp    *http.Response
	respErr error
	done    bool
	final   error
}

type respResult struct {
	resp *http.Response
	err  error
}

// NewStream starts a call to fullMethod, such as "/pkg.Service/Method". The
// context carries the deadline, cancellation, and outgoing metadata.
func (cc *ClientConn) NewStream(ctx context.Context, fullMethod string) (ClientStream, error) {
	cs, err := cc.newStream(ctx, fullMethod)
	if err != nil {
		return nil, err
	}
	return cs, nil
}

func (cc *ClientConn) newStream(ctx context.Context, fullMethod string) (*clientStream, error) {
	var timeout string
	if dl, ok := ctx.Deadline(); ok {
		rem := time.Until(dl)
		if rem <= 0 {
			return nil, Errorf(DeadlineExceeded, "deadline exceeded before call started")
		}
		timeout = encodeTimeout(rem)
	}

	callCtx, cancel := context.WithCancel(ctx)
	pr, pw := io.Pipe()
	req, err := http.NewRequestWithContext(callCtx, http.MethodPost, cc.scheme+"://"+cc.target+fullMethod, pr)
	if err != nil {
		cancel()
		pw.Close()
		return nil, Errorf(Internal, "build request: %v", err)
	}
	req.Header.Set("Content-Type", "application/grpc")
	req.Header.Set("Te", "trailers")
	if timeout != "" {
		req.Header.Set("Grpc-Timeout", timeout)
	}
	if md, ok := FromOutgoingContext(ctx); ok {
		for k, vs := range md {
			if reservedHeader(k) {
				continue
			}
			for _, v := range vs {
				req.Header.Add(k, encodeValue(k, v))
			}
		}
	}

	cs := &clientStream{
		cc:     cc,
		ctx:    callCtx,
		cancel: cancel,
		pw:     pw,
		respc:  make(chan respResult, 1),
	}
	// If the call is cancelled, unblock any SendMsg that is waiting on the pipe.
	go func() {
		<-callCtx.Done()
		pw.CloseWithError(callCtx.Err())
	}()
	go func() {
		resp, err := cc.hc.Do(req)
		if err != nil {
			// No stream was made, so nothing will read the request body. Fail the
			// pipe, or a SendMsg waiting on it blocks forever.
			pr.CloseWithError(err)
		}
		cs.respc <- respResult{resp, err}
	}()
	return cs, nil
}

// waitResponse waits for the response headers, once.
func (cs *clientStream) waitResponse() error {
	if !cs.recvd {
		cs.recvd = true
		r := <-cs.respc
		switch {
		case r.err != nil:
			cs.respErr = transportError(cs.ctx, r.err)
		case r.resp.StatusCode != http.StatusOK:
			r.resp.Body.Close()
			cs.respErr = Errorf(Internal, "unexpected HTTP status %d", r.resp.StatusCode)
		default:
			cs.resp = r.resp
		}
	}
	return cs.respErr
}

// transportError turns a failed round trip into a status. A cancelled or
// expired context keeps its own code.
func transportError(ctx context.Context, err error) error {
	if ctx.Err() != nil {
		code, msg := FromError(ctx.Err())
		return &Error{Code: code, Message: msg}
	}
	return Errorf(Unavailable, "transport: %v", err)
}

func (cs *clientStream) Header() (MD, error) {
	if err := cs.waitResponse(); err != nil {
		return nil, err
	}
	md := MD{}
	for k, vs := range cs.resp.Header {
		key := strings.ToLower(k)
		if reservedHeader(key) {
			continue
		}
		for _, v := range vs {
			if dv, ok := decodeValue(key, v); ok {
				md.Append(key, dv)
			}
		}
	}
	return md, nil
}

func (cs *clientStream) Trailer() MD {
	md := MD{}
	if cs.resp == nil {
		return md
	}
	for k, vs := range cs.resp.Trailer {
		key := strings.ToLower(k)
		if reservedHeader(key) {
			continue
		}
		for _, v := range vs {
			if dv, ok := decodeValue(key, v); ok {
				md.Append(key, dv)
			}
		}
	}
	return md
}

// SendMsg does not wait for the response, which would deadlock a bidi stream.
// Errors from the server surface from RecvMsg.
func (cs *clientStream) SendMsg(m any) error {
	data, err := cs.cc.codec.Marshal(m)
	if err != nil {
		return Errorf(Internal, "marshal request: %v", err)
	}
	return writeFrame(cs.pw, data)
}

func (cs *clientStream) CloseSend() error {
	var err error
	cs.closed.Do(func() { err = cs.pw.Close() })
	return err
}

// recv reads one raw response message. It returns io.EOF when the call has
// ended with OK.
func (cs *clientStream) recv() ([]byte, error) {
	if cs.done {
		return nil, cs.final
	}
	if err := cs.waitResponse(); err != nil {
		cs.finish(err)
		return nil, err
	}
	data, err := readFrame(cs.resp.Body, cs.cc.maxRecv)
	if err == nil {
		return data, nil
	}
	if errors.Is(err, io.EOF) {
		cs.finish(cs.status())
	} else {
		cs.finish(err)
	}
	return nil, cs.final
}

// status reads the final status once the body has ended. It checks the headers
// first, for a trailers-only response, and then the trailers.
func (cs *clientStream) status() error {
	raw := cs.resp.Header.Get("Grpc-Status")
	if raw == "" {
		raw = cs.resp.Trailer.Get("Grpc-Status")
	}
	if raw == "" {
		return Errorf(Internal, "response ended without grpc-status")
	}
	n, err := strconv.ParseUint(raw, 10, 32)
	if err != nil {
		return Errorf(Internal, "bad grpc-status %q", raw)
	}
	if Code(n) == OK {
		return io.EOF
	}
	msg := cs.resp.Header.Get("Grpc-Message")
	if msg == "" {
		msg = cs.resp.Trailer.Get("Grpc-Message")
	}
	return &Error{Code: Code(n), Message: decodeMessage(msg)}
}

// finish records the final result and releases the call's resources.
func (cs *clientStream) finish(err error) {
	cs.done = true
	cs.final = err
	cs.cancel()
}

func (cs *clientStream) RecvMsg(m any) error {
	data, err := cs.recv()
	if err != nil {
		return err
	}
	if err := cs.cc.codec.Unmarshal(data, m); err != nil {
		cs.finish(Errorf(Internal, "unmarshal response: %v", err))
		return cs.final
	}
	return nil
}

// Invoke makes a unary call: one request, one response.
func (cc *ClientConn) Invoke(ctx context.Context, fullMethod string, req, reply any) error {
	cs, err := cc.newStream(ctx, fullMethod)
	if err != nil {
		return err
	}
	if err := cs.SendMsg(req); err != nil {
		// The server may have ended the call early. Its status is the better
		// answer, so read it before returning the send error.
		if _, rerr := cs.recv(); rerr != nil && !errors.Is(rerr, io.EOF) {
			return rerr
		}
		return err
	}
	if err := cs.CloseSend(); err != nil {
		return err
	}
	if err := cs.RecvMsg(reply); err != nil {
		if errors.Is(err, io.EOF) {
			return Errorf(Internal, "unary call ended without a response")
		}
		return err
	}
	// The status comes after the one response. A second message is an error.
	if _, err := cs.recv(); err == nil {
		return Errorf(Internal, "unary call got more than one response")
	} else if !errors.Is(err, io.EOF) {
		return err
	}
	return nil
}

var _ ClientStream = (*clientStream)(nil)
