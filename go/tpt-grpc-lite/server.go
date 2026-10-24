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
	"time"

	"golang.org/x/net/http2"
	"golang.org/x/net/http2/h2c"
)

// ServerStream is what a handler uses to talk to one call.
type ServerStream interface {
	Context() context.Context
	SendMsg(m any) error
	RecvMsg(m any) error
	// SetHeader sets response headers. It must be called before the first
	// SendMsg, and it returns an error after that.
	SetHeader(md MD) error
	// SetTrailer sets trailing metadata, sent after the last message.
	SetTrailer(md MD)
}

// StreamHandler serves one method. It returns the status to send as the error.
// A nil return is OK.
type StreamHandler func(stream ServerStream) error

// Server serves gRPC methods over HTTP/2.
//
// Register handlers with Handle before calling Serve or ServeTLS.
type Server struct {
	codec    Codec
	maxRecv  int
	handlers map[string]StreamHandler
	hs       *http.Server
}

// ServerOption configures a Server.
type ServerOption func(*Server)

// WithServerCodec sets the message codec. The default is ProtoCodec.
func WithServerCodec(c Codec) ServerOption {
	return func(s *Server) { s.codec = c }
}

// WithServerMaxReceiveMessageSize sets the largest request message accepted.
func WithServerMaxReceiveMessageSize(n int) ServerOption {
	return func(s *Server) { s.maxRecv = n }
}

// NewServer returns a Server with no methods registered.
func NewServer(opts ...ServerOption) *Server {
	s := &Server{
		codec:    ProtoCodec{},
		maxRecv:  DefaultMaxMessageSize,
		handlers: make(map[string]StreamHandler),
	}
	for _, o := range opts {
		o(s)
	}
	return s
}

// Handle registers h for fullMethod, such as "/pkg.Service/Method".
func (s *Server) Handle(fullMethod string, h StreamHandler) {
	s.handlers[fullMethod] = h
}

// Serve accepts cleartext HTTP/2 (h2c) connections on lis until it is closed.
// Use ServeTLS in production.
func (s *Server) Serve(lis net.Listener) error {
	s.hs = &http.Server{
		Handler:           h2c.NewHandler(s, &http2.Server{}),
		ReadHeaderTimeout: 10 * time.Second,
	}
	return serveResult(s.hs.Serve(lis))
}

// ServeTLS accepts HTTP/2 over TLS on lis. cfg must hold the server certificate.
// Set ClientAuth in cfg to require client certificates (mTLS).
func (s *Server) ServeTLS(lis net.Listener, cfg *tls.Config) error {
	s.hs = &http.Server{
		Handler:           s,
		TLSConfig:         cfg,
		ReadHeaderTimeout: 10 * time.Second,
	}
	return serveResult(s.hs.ServeTLS(lis, "", ""))
}

// Stop closes all connections at once and cancels the calls in progress.
func (s *Server) Stop() {
	if s.hs != nil {
		_ = s.hs.Close()
	}
}

// GracefulStop stops accepting calls and waits for those in progress to finish.
func (s *Server) GracefulStop() {
	if s.hs != nil {
		_ = s.hs.Shutdown(context.Background())
	}
}

func serveResult(err error) error {
	if errors.Is(err, http.ErrServerClosed) {
		return nil
	}
	return err
}

// ServeHTTP handles one gRPC call. It is exported so the Server can be mounted
// in an existing HTTP/2 server.
func (s *Server) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		w.WriteHeader(http.StatusMethodNotAllowed)
		return
	}
	if !isGRPCContentType(r.Header.Get("Content-Type")) {
		w.WriteHeader(http.StatusUnsupportedMediaType)
		return
	}

	ctx := r.Context()
	if v := r.Header.Get("Grpc-Timeout"); v != "" {
		d, err := parseTimeout(v)
		if err != nil {
			writeTrailersOnly(w, InvalidArgument, err.Error())
			return
		}
		var cancel context.CancelFunc
		ctx, cancel = context.WithTimeout(ctx, d)
		defer cancel()
	}

	if enc := r.Header.Get("Grpc-Encoding"); enc != "" && enc != "identity" {
		writeTrailersOnly(w, Unimplemented, "unsupported grpc-encoding "+enc)
		return
	}

	h, ok := s.handlers[r.URL.Path]
	if !ok {
		writeTrailersOnly(w, Unimplemented, "unknown method "+r.URL.Path)
		return
	}

	ctx = withIncoming(ctx, incomingMD(r.Header))
	st := &serverStream{
		ctx:     ctx,
		w:       w,
		rc:      http.NewResponseController(w),
		body:    r.Body,
		codec:   s.codec,
		maxRecv: s.maxRecv,
	}
	st.finish(h(st))
}

// incomingMD turns request headers into metadata, skipping protocol headers.
// A malformed binary value is dropped, not passed on.
func incomingMD(h http.Header) MD {
	md := MD{}
	for k, vs := range h {
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

func isGRPCContentType(ct string) bool {
	return ct == "application/grpc" || strings.HasPrefix(ct, "application/grpc+") ||
		strings.HasPrefix(ct, "application/grpc;")
}

// writeTrailersOnly sends a response that carries only the status, in the
// headers. The spec uses this for errors that come before any message.
func writeTrailersOnly(w http.ResponseWriter, code Code, msg string) {
	h := w.Header()
	h.Set("Content-Type", "application/grpc")
	h.Set("Grpc-Status", strconv.Itoa(int(code)))
	if msg != "" {
		h.Set("Grpc-Message", encodeMessage(msg))
	}
	w.WriteHeader(http.StatusOK)
}

// serverStream implements ServerStream over one HTTP exchange.
type serverStream struct {
	ctx        context.Context
	w          http.ResponseWriter
	rc         *http.ResponseController
	body       io.ReadCloser
	codec      Codec
	maxRecv    int
	headerSent bool
	header     MD
	trailer    MD
}

func (st *serverStream) Context() context.Context { return st.ctx }

func (st *serverStream) SetHeader(md MD) error {
	if st.headerSent {
		return Errorf(Internal, "SetHeader after the response has started")
	}
	st.header = appendMD(st.header, md)
	return nil
}

func (st *serverStream) SetTrailer(md MD) {
	st.trailer = appendMD(st.trailer, md)
}

// appendMD merges src into dst, creating dst if it is nil.
func appendMD(dst, src MD) MD {
	if dst == nil {
		dst = MD{}
	}
	for k, vs := range src {
		for _, v := range vs {
			dst.Append(k, v)
		}
	}
	return dst
}

// writeMD adds metadata to h as header fields. Protocol headers are skipped.
func writeMD(h http.Header, md MD) {
	for k, vs := range md {
		if reservedHeader(k) {
			continue
		}
		for _, v := range vs {
			h.Add(k, encodeValue(k, v))
		}
	}
}

// sendHeaders writes the response headers once, and declares the status
// trailers that finish will set. Trailers must be declared before the headers
// are written. Custom trailers use http.TrailerPrefix, which needs no
// declaration.
func (st *serverStream) sendHeaders() {
	if st.headerSent {
		return
	}
	h := st.w.Header()
	h.Set("Content-Type", "application/grpc")
	h.Add("Trailer", "Grpc-Status")
	h.Add("Trailer", "Grpc-Message")
	writeMD(h, st.header)
	st.w.WriteHeader(http.StatusOK)
	st.headerSent = true
}

func (st *serverStream) SendMsg(m any) error {
	data, err := st.codec.Marshal(m)
	if err != nil {
		return Errorf(Internal, "marshal response: %v", err)
	}
	st.sendHeaders()
	if err := writeFrame(st.w, data); err != nil {
		return err
	}
	return st.rc.Flush()
}

func (st *serverStream) RecvMsg(m any) error {
	data, err := readFrame(st.body, st.maxRecv)
	if err != nil {
		return err
	}
	if err := st.codec.Unmarshal(data, m); err != nil {
		return Errorf(InvalidArgument, "unmarshal request: %v", err)
	}
	return nil
}

// finish sends the final status. If nothing was sent yet, the status goes in
// the headers (trailers-only). Otherwise it goes in the declared trailers.
func (st *serverStream) finish(err error) {
	code, msg := FromError(err)
	if err == nil {
		code, msg = OK, ""
	}
	if !st.headerSent {
		writeMD(st.w.Header(), st.header)
		writeMD(st.w.Header(), st.trailer)
		writeTrailersOnly(st.w, code, msg)
		return
	}
	h := st.w.Header()
	for k, vs := range st.trailer {
		if reservedHeader(k) {
			continue
		}
		for _, v := range vs {
			h.Add(http.TrailerPrefix+k, encodeValue(k, v))
		}
	}
	h.Set("Grpc-Status", strconv.Itoa(int(code)))
	if msg != "" {
		h.Set("Grpc-Message", encodeMessage(msg))
	}
}

// Unary adapts a function over one request and one response into a
// StreamHandler. It rejects a call that sends zero or several requests.
func Unary[Req, Resp any](fn func(ctx context.Context, req *Req) (*Resp, error)) StreamHandler {
	return func(st ServerStream) error {
		req := new(Req)
		if err := st.RecvMsg(req); err != nil {
			if errors.Is(err, io.EOF) {
				return Errorf(InvalidArgument, "missing request")
			}
			return err
		}
		extra := new(Req)
		switch err := st.RecvMsg(extra); {
		case err == nil:
			return Errorf(InvalidArgument, "unary call sent more than one request")
		case !errors.Is(err, io.EOF):
			return err
		}
		resp, err := fn(st.Context(), req)
		if err != nil {
			return err
		}
		if resp == nil {
			return Errorf(Internal, "handler returned no response")
		}
		return st.SendMsg(resp)
	}
}

var (
	_ ServerStream = (*serverStream)(nil)
	_ http.Handler = (*Server)(nil)
)
