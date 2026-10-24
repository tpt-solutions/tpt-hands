// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package grpclite

import (
	"bytes"
	"context"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/tls"
	"crypto/x509"
	"crypto/x509/pkix"
	"errors"
	"fmt"
	"io"
	"math/big"
	"net"
	"strings"
	"sync"
	"testing"
	"time"

	"google.golang.org/protobuf/types/known/wrapperspb"
)

// ---- wire-level units ----

func TestFrameRoundTrip(t *testing.T) {
	var buf bytes.Buffer
	for _, p := range [][]byte{nil, []byte("a"), bytes.Repeat([]byte{7}, 1000)} {
		if err := writeFrame(&buf, p); err != nil {
			t.Fatal(err)
		}
	}
	for _, want := range [][]byte{{}, []byte("a"), bytes.Repeat([]byte{7}, 1000)} {
		got, err := readFrame(&buf, DefaultMaxMessageSize)
		if err != nil {
			t.Fatal(err)
		}
		if !bytes.Equal(got, want) {
			t.Fatalf("got %d bytes, want %d", len(got), len(want))
		}
	}
	if _, err := readFrame(&buf, DefaultMaxMessageSize); !errors.Is(err, io.EOF) {
		t.Fatalf("want clean EOF, got %v", err)
	}
}

func TestFrameWireFormat(t *testing.T) {
	var buf bytes.Buffer
	_ = writeFrame(&buf, []byte("hi"))
	want := []byte{0, 0, 0, 0, 2, 'h', 'i'}
	if !bytes.Equal(buf.Bytes(), want) {
		t.Fatalf("frame = %v, want %v", buf.Bytes(), want)
	}
}

func TestFrameRejectsCompressionAndOversize(t *testing.T) {
	_, err := readFrame(bytes.NewReader([]byte{1, 0, 0, 0, 0}), 10)
	if c, _ := FromError(err); c != Unimplemented {
		t.Errorf("compressed flag: code %d, want Unimplemented", c)
	}
	_, err = readFrame(bytes.NewReader([]byte{0, 0, 0, 0, 20}), 10)
	if c, _ := FromError(err); c != ResourceExhausted {
		t.Errorf("oversize: code %d, want ResourceExhausted", c)
	}
}

func TestFrameTruncatedIsInternal(t *testing.T) {
	_, err := readFrame(bytes.NewReader([]byte{0, 0, 0}), 10)
	if c, _ := FromError(err); c != Internal {
		t.Errorf("truncated header: code %d, want Internal", c)
	}
	_, err = readFrame(bytes.NewReader([]byte{0, 0, 0, 0, 5, 'a'}), 10)
	if c, _ := FromError(err); c != Internal {
		t.Errorf("truncated body: code %d, want Internal", c)
	}
}

func TestTimeoutEncodeParseRoundTrip(t *testing.T) {
	cases := []time.Duration{
		0, time.Nanosecond, 250 * time.Microsecond, 100 * time.Millisecond,
		5 * time.Second, 90 * time.Minute, 30 * 24 * time.Hour,
	}
	for _, d := range cases {
		s := encodeTimeout(d)
		got, err := parseTimeout(s)
		if err != nil {
			t.Fatalf("%v -> %q: %v", d, s, err)
		}
		if got > d {
			t.Errorf("%v -> %q -> %v: deadline was extended", d, s, got)
		}
		if len(s) > 9 {
			t.Errorf("%q is longer than the 8 digits + unit the spec allows", s)
		}
	}
}

func TestTimeoutKnownValues(t *testing.T) {
	// 1e8 ns has nine digits, so the finest unit that fits is microseconds.
	if s := encodeTimeout(100 * time.Millisecond); s != "100000u" {
		t.Errorf("100ms = %q", s)
	}
	if s := encodeTimeout(5 * time.Second); s != "5000000u" {
		t.Errorf("5s = %q", s)
	}
	if _, err := parseTimeout("12X"); err == nil {
		t.Error("unknown unit accepted")
	}
	if _, err := parseTimeout("123456789m"); err == nil {
		t.Error("nine digits accepted")
	}
}

func TestMessageEscaping(t *testing.T) {
	for _, msg := range []string{"plain", "héllo 100% ok", "line\nbreak", ""} {
		enc := encodeMessage(msg)
		for i := 0; i < len(enc); i++ {
			if enc[i] < 0x20 || enc[i] > 0x7e {
				t.Fatalf("encoded %q contains byte %#x", msg, enc[i])
			}
		}
		if got := decodeMessage(enc); got != msg {
			t.Errorf("decode(encode(%q)) = %q", msg, got)
		}
	}
}

func TestBinaryMetadataIsBase64(t *testing.T) {
	v, ok := decodeValue("x-blob-bin", encodeValue("x-blob-bin", "\x00\xff raw"))
	if !ok || v != "\x00\xff raw" {
		t.Errorf("binary round trip = %q, %v", v, ok)
	}
	if v, ok := decodeValue("x-blob-bin", "bm8"); !ok || v != "no" {
		t.Errorf("unpadded base64 = %q, %v", v, ok)
	}
	if _, ok := decodeValue("x-blob-bin", "!!!"); ok {
		t.Error("bad base64 accepted")
	}
}

func TestMDIsCaseInsensitiveAndCopied(t *testing.T) {
	md := NewMD("X-User", "ann", "x-user", "bob")
	if got := md.Get("X-USER"); len(got) != 2 {
		t.Fatalf("Get = %v", got)
	}
	cp := md.Copy()
	cp.Append("x-user", "carol")
	if len(md.Get("x-user")) != 2 {
		t.Error("Copy shares slices with the original")
	}
}

// ---- end to end over HTTP/2 ----

// fixtureServer registers the methods the tests call.
func fixtureServer(t testing.TB, opts ...ServerOption) *Server {
	t.Helper()
	s := NewServer(opts...)

	s.Handle("/test.Echo/Unary", Unary(func(ctx context.Context, req *wrapperspb.StringValue) (*wrapperspb.StringValue, error) {
		return wrapperspb.String("echo:" + req.GetValue()), nil
	}))

	s.Handle("/test.Echo/Fail", Unary(func(ctx context.Context, req *wrapperspb.StringValue) (*wrapperspb.StringValue, error) {
		return nil, Errorf(NotFound, "no such thing: %s", req.GetValue())
	}))

	s.Handle("/test.Echo/Slow", Unary(func(ctx context.Context, req *wrapperspb.StringValue) (*wrapperspb.StringValue, error) {
		<-ctx.Done()
		return nil, ctx.Err()
	}))

	s.Handle("/test.Echo/Meta", Unary(func(ctx context.Context, req *wrapperspb.StringValue) (*wrapperspb.StringValue, error) {
		md, _ := FromIncomingContext(ctx)
		return wrapperspb.String(strings.Join(md.Get("x-user"), ",") + "|" + strings.Join(md.Get("x-blob-bin"), ",")), nil
	}))

	s.Handle("/test.Echo/Headers", func(st ServerStream) error {
		if err := st.SetHeader(NewMD("x-server", "h1")); err != nil {
			return err
		}
		var req wrapperspb.StringValue
		if err := st.RecvMsg(&req); err != nil {
			return err
		}
		if err := st.SendMsg(wrapperspb.String(req.GetValue())); err != nil {
			return err
		}
		st.SetTrailer(NewMD("x-trail", "t1"))
		return nil
	})

	// Count: one request n, then n messages 1..n.
	s.Handle("/test.Echo/Count", func(st ServerStream) error {
		var req wrapperspb.Int64Value
		if err := st.RecvMsg(&req); err != nil {
			return err
		}
		for i := int64(1); i <= req.GetValue(); i++ {
			if err := st.SendMsg(wrapperspb.Int64(i)); err != nil {
				return err
			}
		}
		return nil
	})

	// Sum: reads messages until the client closes, then sends the total.
	s.Handle("/test.Echo/Sum", func(st ServerStream) error {
		var total int64
		for {
			var req wrapperspb.Int64Value
			err := st.RecvMsg(&req)
			if errors.Is(err, io.EOF) {
				break
			}
			if err != nil {
				return err
			}
			total += req.GetValue()
		}
		return st.SendMsg(wrapperspb.Int64(total))
	})

	// Upper: echoes each message in upper case, interleaved.
	s.Handle("/test.Echo/Upper", func(st ServerStream) error {
		for {
			var req wrapperspb.StringValue
			err := st.RecvMsg(&req)
			if errors.Is(err, io.EOF) {
				return nil
			}
			if err != nil {
				return err
			}
			if err := st.SendMsg(wrapperspb.String(strings.ToUpper(req.GetValue()))); err != nil {
				return err
			}
		}
	})

	return s
}

// startPlain serves s on loopback and returns the address and a stop function.
func startPlain(t testing.TB, s *Server) (string, func()) {
	t.Helper()
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	go func() { _ = s.Serve(lis) }()
	return lis.Addr().String(), s.Stop
}

func TestUnaryRoundTrip(t *testing.T) {
	addr, stop := startPlain(t, fixtureServer(t))
	defer stop()
	cc := Dial(addr)
	defer cc.Close()

	var out wrapperspb.StringValue
	if err := cc.Invoke(context.Background(), "/test.Echo/Unary", wrapperspb.String("hi"), &out); err != nil {
		t.Fatal(err)
	}
	if out.GetValue() != "echo:hi" {
		t.Errorf("got %q", out.GetValue())
	}
}

func TestErrorStatusAndMessage(t *testing.T) {
	addr, stop := startPlain(t, fixtureServer(t))
	defer stop()
	cc := Dial(addr)
	defer cc.Close()

	var out wrapperspb.StringValue
	err := cc.Invoke(context.Background(), "/test.Echo/Fail", wrapperspb.String("héllo 100%"), &out)
	code, msg := FromError(err)
	if code != NotFound {
		t.Fatalf("code = %d, err = %v", code, err)
	}
	if msg != "no such thing: héllo 100%" {
		t.Errorf("message = %q", msg)
	}
}

func TestUnknownMethodIsUnimplemented(t *testing.T) {
	addr, stop := startPlain(t, fixtureServer(t))
	defer stop()
	cc := Dial(addr)
	defer cc.Close()

	var out wrapperspb.StringValue
	err := cc.Invoke(context.Background(), "/test.Echo/Nope", wrapperspb.String("x"), &out)
	if code, _ := FromError(err); code != Unimplemented {
		t.Errorf("code = %d, err = %v", code, err)
	}
}

func TestDeadlinePropagatesToServer(t *testing.T) {
	addr, stop := startPlain(t, fixtureServer(t))
	defer stop()
	cc := Dial(addr)
	defer cc.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 150*time.Millisecond)
	defer cancel()
	start := time.Now()
	var out wrapperspb.StringValue
	err := cc.Invoke(ctx, "/test.Echo/Slow", wrapperspb.String("x"), &out)
	if code, _ := FromError(err); code != DeadlineExceeded {
		t.Fatalf("code = %d, err = %v", code, err)
	}
	if elapsed := time.Since(start); elapsed > 2*time.Second {
		t.Errorf("call took %v; the deadline was not enforced", elapsed)
	}
}

func TestExpiredDeadlineFailsBeforeSending(t *testing.T) {
	addr, stop := startPlain(t, fixtureServer(t))
	defer stop()
	cc := Dial(addr)
	defer cc.Close()

	ctx, cancel := context.WithDeadline(context.Background(), time.Now().Add(-time.Second))
	defer cancel()
	var out wrapperspb.StringValue
	err := cc.Invoke(ctx, "/test.Echo/Unary", wrapperspb.String("x"), &out)
	if code, _ := FromError(err); code != DeadlineExceeded {
		t.Errorf("code = %d, err = %v", code, err)
	}
}

func TestCancelEndsCall(t *testing.T) {
	addr, stop := startPlain(t, fixtureServer(t))
	defer stop()
	cc := Dial(addr)
	defer cc.Close()

	ctx, cancel := context.WithCancel(context.Background())
	go func() {
		time.Sleep(100 * time.Millisecond)
		cancel()
	}()
	var out wrapperspb.StringValue
	err := cc.Invoke(ctx, "/test.Echo/Slow", wrapperspb.String("x"), &out)
	if code, _ := FromError(err); code != Canceled {
		t.Errorf("code = %d, err = %v", code, err)
	}
}

func TestServerStreaming(t *testing.T) {
	addr, stop := startPlain(t, fixtureServer(t))
	defer stop()
	cc := Dial(addr)
	defer cc.Close()

	cs, err := cc.NewStream(context.Background(), "/test.Echo/Count")
	if err != nil {
		t.Fatal(err)
	}
	if err := cs.SendMsg(wrapperspb.Int64(5)); err != nil {
		t.Fatal(err)
	}
	if err := cs.CloseSend(); err != nil {
		t.Fatal(err)
	}
	var got []int64
	for {
		var v wrapperspb.Int64Value
		err := cs.RecvMsg(&v)
		if errors.Is(err, io.EOF) {
			break
		}
		if err != nil {
			t.Fatal(err)
		}
		got = append(got, v.GetValue())
	}
	if fmt.Sprint(got) != "[1 2 3 4 5]" {
		t.Errorf("got %v", got)
	}
}

func TestClientStreaming(t *testing.T) {
	addr, stop := startPlain(t, fixtureServer(t))
	defer stop()
	cc := Dial(addr)
	defer cc.Close()

	cs, err := cc.NewStream(context.Background(), "/test.Echo/Sum")
	if err != nil {
		t.Fatal(err)
	}
	for i := int64(1); i <= 10; i++ {
		if err := cs.SendMsg(wrapperspb.Int64(i)); err != nil {
			t.Fatal(err)
		}
	}
	if err := cs.CloseSend(); err != nil {
		t.Fatal(err)
	}
	var total wrapperspb.Int64Value
	if err := cs.RecvMsg(&total); err != nil {
		t.Fatal(err)
	}
	if total.GetValue() != 55 {
		t.Errorf("sum = %d, want 55", total.GetValue())
	}
	if err := cs.RecvMsg(&total); !errors.Is(err, io.EOF) {
		t.Errorf("after the response, want io.EOF, got %v", err)
	}
}

func TestBidiStreamingInterleaves(t *testing.T) {
	addr, stop := startPlain(t, fixtureServer(t))
	defer stop()
	cc := Dial(addr)
	defer cc.Close()

	cs, err := cc.NewStream(context.Background(), "/test.Echo/Upper")
	if err != nil {
		t.Fatal(err)
	}
	words := []string{"alpha", "beta", "gamma"}
	for _, w := range words {
		// Each reply must arrive before the next request is sent.
		if err := cs.SendMsg(wrapperspb.String(w)); err != nil {
			t.Fatal(err)
		}
		var out wrapperspb.StringValue
		if err := cs.RecvMsg(&out); err != nil {
			t.Fatal(err)
		}
		if out.GetValue() != strings.ToUpper(w) {
			t.Errorf("got %q for %q", out.GetValue(), w)
		}
	}
	if err := cs.CloseSend(); err != nil {
		t.Fatal(err)
	}
	var out wrapperspb.StringValue
	if err := cs.RecvMsg(&out); !errors.Is(err, io.EOF) {
		t.Errorf("want clean end, got %v", err)
	}
}

func TestMetadataAndHeadersRoundTrip(t *testing.T) {
	addr, stop := startPlain(t, fixtureServer(t))
	defer stop()
	cc := Dial(addr)
	defer cc.Close()

	ctx := NewOutgoingContext(context.Background(), NewMD(
		"x-user", "ann",
		"x-blob-bin", "\x01\x02",
	))
	var out wrapperspb.StringValue
	if err := cc.Invoke(ctx, "/test.Echo/Meta", wrapperspb.String("x"), &out); err != nil {
		t.Fatal(err)
	}
	if out.GetValue() != "ann|\x01\x02" {
		t.Errorf("server saw %q", out.GetValue())
	}

	cs, err := cc.NewStream(ctx, "/test.Echo/Headers")
	if err != nil {
		t.Fatal(err)
	}
	if err := cs.SendMsg(wrapperspb.String("x")); err != nil {
		t.Fatal(err)
	}
	if err := cs.CloseSend(); err != nil {
		t.Fatal(err)
	}
	hdr, err := cs.Header()
	if err != nil {
		t.Fatal(err)
	}
	if got := hdr.Get("x-server"); len(got) != 1 || got[0] != "h1" {
		t.Errorf("header x-server = %v", got)
	}
	var v wrapperspb.StringValue
	if err := cs.RecvMsg(&v); err != nil {
		t.Fatal(err)
	}
	if err := cs.RecvMsg(&v); !errors.Is(err, io.EOF) {
		t.Fatalf("want end of call, got %v", err)
	}
	if got := cs.Trailer().Get("x-trail"); len(got) != 1 || got[0] != "t1" {
		t.Errorf("trailer x-trail = %v", got)
	}
}

func TestMessageLimitIsEnforced(t *testing.T) {
	addr, stop := startPlain(t, fixtureServer(t, WithServerMaxReceiveMessageSize(16)))
	defer stop()
	cc := Dial(addr)
	defer cc.Close()

	big := wrapperspb.String(strings.Repeat("x", 100))
	var out wrapperspb.StringValue
	err := cc.Invoke(context.Background(), "/test.Echo/Unary", big, &out)
	if code, _ := FromError(err); code != ResourceExhausted {
		t.Errorf("code = %d, err = %v", code, err)
	}
}

func TestConcurrentCallsShareConnections(t *testing.T) {
	addr, stop := startPlain(t, fixtureServer(t))
	defer stop()
	cc := Dial(addr)
	defer cc.Close()

	const n = 64
	var wg sync.WaitGroup
	errs := make(chan error, n)
	for i := 0; i < n; i++ {
		wg.Add(1)
		go func(i int) {
			defer wg.Done()
			in := fmt.Sprintf("m%d", i)
			var out wrapperspb.StringValue
			if err := cc.Invoke(context.Background(), "/test.Echo/Unary", wrapperspb.String(in), &out); err != nil {
				errs <- err
				return
			}
			if out.GetValue() != "echo:"+in {
				errs <- fmt.Errorf("call %d got %q", i, out.GetValue())
			}
		}(i)
	}
	wg.Wait()
	close(errs)
	for err := range errs {
		t.Error(err)
	}
}

func TestReconnectAfterServerRestart(t *testing.T) {
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	addr := lis.Addr().String()
	s1 := fixtureServer(t)
	go func() { _ = s1.Serve(lis) }()

	cc := Dial(addr)
	defer cc.Close()
	var out wrapperspb.StringValue
	if err := cc.Invoke(context.Background(), "/test.Echo/Unary", wrapperspb.String("a"), &out); err != nil {
		t.Fatal(err)
	}

	s1.Stop()
	lis2, err := net.Listen("tcp", addr)
	if err != nil {
		t.Skipf("could not rebind %s: %v", addr, err)
	}
	s2 := fixtureServer(t)
	go func() { _ = s2.Serve(lis2) }()
	defer s2.Stop()

	var retry error
	for i := 0; i < 20; i++ {
		retry = cc.Invoke(context.Background(), "/test.Echo/Unary", wrapperspb.String("b"), &out)
		if retry == nil {
			break
		}
		time.Sleep(50 * time.Millisecond)
	}
	if retry != nil {
		t.Fatalf("call after restart: %v", retry)
	}
	if out.GetValue() != "echo:b" {
		t.Errorf("got %q", out.GetValue())
	}
}

// ---- TLS and mTLS ----

// selfSigned returns one certificate and key, valid for loopback, which the
// test trusts as its own CA.
func selfSigned(t *testing.T) tls.Certificate {
	t.Helper()
	key, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	tmpl := &x509.Certificate{
		SerialNumber: big.NewInt(1),
		Subject:      pkix.Name{CommonName: "tpt-grpc-lite test"},
		NotBefore:    time.Now().Add(-time.Hour),
		NotAfter:     time.Now().Add(time.Hour),
		KeyUsage:     x509.KeyUsageDigitalSignature | x509.KeyUsageCertSign,
		ExtKeyUsage:  []x509.ExtKeyUsage{x509.ExtKeyUsageServerAuth, x509.ExtKeyUsageClientAuth},
		IsCA:         true,
		DNSNames:     []string{"localhost"},
		IPAddresses:  []net.IP{net.ParseIP("127.0.0.1")},
	}
	der, err := x509.CreateCertificate(rand.Reader, tmpl, tmpl, &key.PublicKey, key)
	if err != nil {
		t.Fatal(err)
	}
	leaf, err := x509.ParseCertificate(der)
	if err != nil {
		t.Fatal(err)
	}
	return tls.Certificate{Certificate: [][]byte{der}, PrivateKey: key, Leaf: leaf}
}

func certPool(c tls.Certificate) *x509.CertPool {
	p := x509.NewCertPool()
	p.AddCert(c.Leaf)
	return p
}

func TestTLSUnaryCall(t *testing.T) {
	cert := selfSigned(t)
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	s := fixtureServer(t)
	go func() {
		_ = s.ServeTLS(lis, &tls.Config{Certificates: []tls.Certificate{cert}, MinVersion: tls.VersionTLS12})
	}()
	defer s.Stop()

	cc := Dial(lis.Addr().String(), WithTLS(&tls.Config{RootCAs: certPool(cert), ServerName: "localhost", MinVersion: tls.VersionTLS12}))
	defer cc.Close()
	var out wrapperspb.StringValue
	if err := cc.Invoke(context.Background(), "/test.Echo/Unary", wrapperspb.String("tls"), &out); err != nil {
		t.Fatal(err)
	}
	if out.GetValue() != "echo:tls" {
		t.Errorf("got %q", out.GetValue())
	}
}

func TestTLSRejectsUntrustedServer(t *testing.T) {
	cert := selfSigned(t)
	other := selfSigned(t)
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	s := fixtureServer(t)
	go func() {
		_ = s.ServeTLS(lis, &tls.Config{Certificates: []tls.Certificate{cert}, MinVersion: tls.VersionTLS12})
	}()
	defer s.Stop()

	cc := Dial(lis.Addr().String(), WithTLS(&tls.Config{RootCAs: certPool(other), ServerName: "localhost", MinVersion: tls.VersionTLS12}))
	defer cc.Close()
	var out wrapperspb.StringValue
	err = cc.Invoke(context.Background(), "/test.Echo/Unary", wrapperspb.String("x"), &out)
	if err == nil {
		t.Fatal("call succeeded against an untrusted server certificate")
	}
}

func TestMutualTLSRequiresClientCertificate(t *testing.T) {
	serverCert := selfSigned(t)
	clientCert := selfSigned(t)
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	s := fixtureServer(t)
	go func() {
		_ = s.ServeTLS(lis, &tls.Config{
			Certificates: []tls.Certificate{serverCert},
			ClientAuth:   tls.RequireAndVerifyClientCert,
			ClientCAs:    certPool(clientCert),
			MinVersion:   tls.VersionTLS12,
		})
	}()
	defer s.Stop()
	addr := lis.Addr().String()

	good := Dial(addr, WithTLS(&tls.Config{
		RootCAs:      certPool(serverCert),
		Certificates: []tls.Certificate{clientCert},
		ServerName:   "localhost",
		MinVersion:   tls.VersionTLS12,
	}))
	defer good.Close()
	var out wrapperspb.StringValue
	if err := good.Invoke(context.Background(), "/test.Echo/Unary", wrapperspb.String("m"), &out); err != nil {
		t.Fatalf("mTLS call with client certificate: %v", err)
	}

	bad := Dial(addr, WithTLS(&tls.Config{
		RootCAs:    certPool(serverCert),
		ServerName: "localhost",
		MinVersion: tls.VersionTLS12,
	}))
	defer bad.Close()
	if err := bad.Invoke(context.Background(), "/test.Echo/Unary", wrapperspb.String("m"), &out); err == nil {
		t.Fatal("call succeeded without a client certificate")
	}
}

// ---- status mapping ----

func TestFromErrorMapsContextAndUnknown(t *testing.T) {
	cases := []struct {
		err  error
		want Code
	}{
		{nil, OK},
		{Errorf(InvalidArgument, "bad"), InvalidArgument},
		{fmt.Errorf("wrapped: %w", Errorf(Aborted, "x")), Aborted},
		{context.DeadlineExceeded, DeadlineExceeded},
		{context.Canceled, Canceled},
		{errors.New("secret internal detail"), Unknown},
	}
	for _, c := range cases {
		if got, _ := FromError(c.err); got != c.want {
			t.Errorf("FromError(%v) = %d, want %d", c.err, got, c.want)
		}
	}
	if _, msg := FromError(errors.New("secret internal detail")); strings.Contains(msg, "secret") {
		t.Error("internal error text leaked to the client")
	}
}
