// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package grpclite

import (
	"context"
	"errors"
	"io"
	"strings"
	"testing"

	"google.golang.org/protobuf/types/known/wrapperspb"
)

// BenchmarkUnaryLatency measures one sequential unary call on a warm
// connection. ns/op is the round-trip time.
func BenchmarkUnaryLatency(b *testing.B) {
	addr, stop := startPlain(b, fixtureServer(b))
	defer stop()
	cc := Dial(addr)
	defer cc.Close()

	req := wrapperspb.String("ping")
	var out wrapperspb.StringValue
	ctx := context.Background()
	if err := cc.Invoke(ctx, "/test.Echo/Unary", req, &out); err != nil {
		b.Fatal(err)
	}

	b.ReportAllocs()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		if err := cc.Invoke(ctx, "/test.Echo/Unary", req, &out); err != nil {
			b.Fatal(err)
		}
	}
}

// BenchmarkUnaryParallel measures unary throughput with many callers sharing
// one connection pool. ns/op is the time per call across all goroutines.
func BenchmarkUnaryParallel(b *testing.B) {
	addr, stop := startPlain(b, fixtureServer(b))
	defer stop()
	cc := Dial(addr)
	defer cc.Close()

	req := wrapperspb.String("ping")
	ctx := context.Background()
	b.ReportAllocs()
	b.ResetTimer()
	b.RunParallel(func(pb *testing.PB) {
		var out wrapperspb.StringValue
		for pb.Next() {
			if err := cc.Invoke(ctx, "/test.Echo/Unary", req, &out); err != nil {
				b.Error(err)
				return
			}
		}
	})
}

// BenchmarkBidiThroughput streams 1 KiB messages through one bidi call and
// reports bytes per second in each direction together.
func BenchmarkBidiThroughput(b *testing.B) {
	addr, stop := startPlain(b, fixtureServer(b))
	defer stop()
	cc := Dial(addr)
	defer cc.Close()

	payload := strings.Repeat("a", 1024)
	cs, err := cc.NewStream(context.Background(), "/test.Echo/Upper")
	if err != nil {
		b.Fatal(err)
	}
	b.SetBytes(int64(len(payload)) * 2)
	b.ReportAllocs()
	b.ResetTimer()

	// Keep a window of messages in flight, so the benchmark measures streaming
	// and not one round trip per message.
	const window = 64
	sent, recvd := 0, 0
	msg := wrapperspb.String(payload)
	for recvd < b.N {
		for sent < b.N && sent-recvd < window {
			if err := cs.SendMsg(msg); err != nil {
				b.Fatal(err)
			}
			sent++
		}
		var out wrapperspb.StringValue
		if err := cs.RecvMsg(&out); err != nil {
			b.Fatal(err)
		}
		recvd++
	}
	if err := cs.CloseSend(); err != nil {
		b.Fatal(err)
	}
	var end wrapperspb.StringValue
	if err := cs.RecvMsg(&end); !errors.Is(err, io.EOF) {
		b.Fatalf("want end of call, got %v", err)
	}
}
