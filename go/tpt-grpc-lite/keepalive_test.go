// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package grpclite

import (
	"context"
	"io"
	"net"
	"sync/atomic"
	"testing"
	"time"

	"google.golang.org/protobuf/types/known/wrapperspb"
)

// freezeProxy forwards TCP to a target. When frozen, it keeps the connection
// open but drops every byte in both directions. That is a silent dead peer: no
// reset, no FIN, and no replies.
type freezeProxy struct {
	lis    net.Listener
	frozen atomic.Bool
}

func newFreezeProxy(t *testing.T, target string) *freezeProxy {
	t.Helper()
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	p := &freezeProxy{lis: lis}
	go func() {
		for {
			c, err := lis.Accept()
			if err != nil {
				return
			}
			up, err := net.Dial("tcp", target)
			if err != nil {
				c.Close()
				continue
			}
			go p.pump(up, c)
			go p.pump(c, up)
		}
	}()
	t.Cleanup(func() { lis.Close() })
	return p
}

func (p *freezeProxy) pump(dst io.WriteCloser, src io.ReadCloser) {
	buf := make([]byte, 32<<10)
	for {
		n, err := src.Read(buf)
		if n > 0 && !p.frozen.Load() {
			if _, werr := dst.Write(buf[:n]); werr != nil {
				return
			}
		}
		if err != nil {
			return
		}
	}
}

func (p *freezeProxy) addr() string { return p.lis.Addr().String() }

// TestKeepaliveDetectsSilentPeer checks that a call on a connection whose peer
// has gone silent fails in bounded time. Without keepalive, it would wait for
// the caller's deadline, which here is 30 seconds.
func TestKeepaliveDetectsSilentPeer(t *testing.T) {
	s := fixtureServer(t)
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	go func() { _ = s.Serve(lis) }()
	t.Cleanup(s.Stop)

	proxy := newFreezeProxy(t, lis.Addr().String())
	cc := Dial(proxy.addr(), WithKeepalive(100*time.Millisecond, 300*time.Millisecond))
	t.Cleanup(func() { cc.Close() })

	// Establish the connection while the peer is still reachable.
	var out wrapperspb.StringValue
	if err := cc.Invoke(context.Background(), "/test.Echo/Unary", wrapperspb.String("warm"), &out); err != nil {
		t.Fatal(err)
	}

	proxy.frozen.Store(true)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	start := time.Now()
	err = cc.Invoke(ctx, "/test.Echo/Unary", wrapperspb.String("lost"), &out)
	elapsed := time.Since(start)

	if err == nil {
		t.Fatal("call succeeded through a frozen peer")
	}
	if code, _ := FromError(err); code == DeadlineExceeded {
		t.Fatalf("call waited for its own deadline (%v); keepalive did not fire", elapsed)
	}
	if elapsed > 10*time.Second {
		t.Errorf("silent peer was detected after %v; expected a few keepalive intervals", elapsed)
	}
}
