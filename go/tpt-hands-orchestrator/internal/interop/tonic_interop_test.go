// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

// Package interop checks the Ack method between tpt-grpc-lite (Go) and the
// tonic spike in tools/grpc-spike (Rust), in both directions.
package interop

import (
	"bufio"
	"context"
	"net"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"

	grpclite "github.com/tpt-solutions/tpt-hands/go/tpt-grpc-lite"
	gen "github.com/tpt-solutions/tpt-hands/go/tpt-hands-orchestrator/internal/gen/tpthandsv1"
)

const ackPath = "/tpt.hands.v1.Ack/Acknowledge"

// repoRoot is the workspace root, four levels up from this package.
func repoRoot(t *testing.T) string {
	t.Helper()
	abs, err := filepath.Abs("../../../..")
	if err != nil {
		t.Fatal(err)
	}
	return abs
}

// buildSpike builds one of the grpc-spike binaries with cargo and returns its
// path. The spike is a standalone crate, so it has its own target directory.
// The test skips when cargo is missing.
func buildSpike(t *testing.T, bin string) string {
	t.Helper()
	if _, err := exec.LookPath("cargo"); err != nil {
		t.Skip("skipping interop: cargo not on PATH")
	}
	spike := filepath.Join(repoRoot(t), "tools", "grpc-spike")
	build := exec.Command("cargo", "build", "-q", "--bin", bin)
	build.Dir = spike
	if out, err := build.CombinedOutput(); err != nil {
		t.Fatalf("cargo build %s: %v\n%s", bin, err, out)
	}
	name := bin
	if runtime.GOOS == "windows" {
		name += ".exe"
	}
	return filepath.Join(spike, "target", "debug", name)
}

func freeAddr(t *testing.T) string {
	t.Helper()
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	defer lis.Close()
	return lis.Addr().String()
}

// TestGoClientToTonicServer sends Acknowledge from grpclite to the Rust server.
func TestGoClientToTonicServer(t *testing.T) {
	bin := buildSpike(t, "ack-server")
	addr := freeAddr(t)

	srv := exec.Command(bin, addr)
	stdout, err := srv.StdoutPipe()
	if err != nil {
		t.Fatal(err)
	}
	if err := srv.Start(); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		_ = srv.Process.Kill()
		_ = srv.Wait()
	})
	line, err := bufio.NewReader(stdout).ReadString('\n')
	if err != nil || !strings.HasPrefix(line, "listening") {
		t.Fatalf("ack-server did not start: %q, %v", line, err)
	}

	cc := grpclite.Dial(addr)
	t.Cleanup(func() { cc.Close() })
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	var ack gen.MorphogenesisAck
	req := &gen.MorphogenesisRequest{ToolId: "hand-0", RequestId: "go-1"}
	if err := cc.Invoke(ctx, ackPath, req, &ack); err != nil {
		t.Fatalf("Acknowledge: %v", err)
	}
	if !ack.GetApproved() || ack.GetRequestId() != "go-1" {
		t.Errorf("ack = approved %v id %q", ack.GetApproved(), ack.GetRequestId())
	}

	// The server's application error must reach the Go client as a status.
	err = cc.Invoke(ctx, ackPath, &gen.MorphogenesisRequest{RequestId: "go-2"}, &ack)
	if code, _ := grpclite.FromError(err); code != grpclite.InvalidArgument {
		t.Errorf("missing tool_id: code %d, err %v", code, err)
	}
}

// TestTonicClientToGoServer sends Acknowledge from the Rust client to a
// grpclite server.
func TestTonicClientToGoServer(t *testing.T) {
	bin := buildSpike(t, "ack-client")

	srv := grpclite.NewServer()
	srv.Handle(ackPath, grpclite.Unary(func(ctx context.Context, req *gen.MorphogenesisRequest) (*gen.MorphogenesisAck, error) {
		if req.GetToolId() == "" {
			return nil, grpclite.Errorf(grpclite.InvalidArgument, "tool_id is required")
		}
		return &gen.MorphogenesisAck{RequestId: req.GetRequestId(), Approved: true}, nil
	}))
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	go func() { _ = srv.Serve(lis) }()
	t.Cleanup(srv.Stop)

	out, err := exec.Command(bin, lis.Addr().String()).CombinedOutput()
	if err != nil {
		t.Fatalf("ack-client against grpclite server: %v\n%s", err, out)
	}
	if !strings.Contains(string(out), "approved interop-1") {
		t.Errorf("unexpected client output: %s", out)
	}
}
