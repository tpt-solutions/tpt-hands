// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

package grpclite

import (
	"bufio"
	"context"
	"errors"
	"fmt"
	"io"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"google.golang.org/protobuf/types/known/wrapperspb"
)

// The interop tests run against grpc-core (the C implementation behind the
// Python grpc package), which is an independent implementation of the same
// wire protocol. They skip when Python with grpcio is not installed. Set
// TPT_GRPC_PYTHON to choose the interpreter.

func pythonInterpreter(t *testing.T) string {
	t.Helper()
	py := os.Getenv("TPT_GRPC_PYTHON")
	if py == "" {
		py = "python"
	}
	if err := exec.Command(py, "-c", "import grpc").Run(); err != nil {
		t.Skipf("skipping interop: %s with grpcio not available: %v", py, err)
	}
	return py
}

func freePort(t *testing.T) string {
	t.Helper()
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	defer lis.Close()
	return lis.Addr().String()
}

func waitForPort(t *testing.T, addr string) {
	t.Helper()
	deadline := time.Now().Add(10 * time.Second)
	for time.Now().Before(deadline) {
		if c, err := net.DialTimeout("tcp", addr, 200*time.Millisecond); err == nil {
			c.Close()
			return
		}
		time.Sleep(50 * time.Millisecond)
	}
	t.Fatalf("nothing listening on %s", addr)
}

// interopDir is the directory holding the Python scripts.
func interopDir(t *testing.T) string {
	t.Helper()
	abs, err := filepath.Abs("interop")
	if err != nil {
		t.Fatal(err)
	}
	return abs
}

// TestInteropGoClientToGrpcCoreServer runs the Go client checks against a
// grpc-core server.
func TestInteropGoClientToGrpcCoreServer(t *testing.T) {
	py := pythonInterpreter(t)
	addr := freePort(t)
	port := addr[strings.LastIndex(addr, ":")+1:]

	srv := exec.Command(py, "server.py", port)
	srv.Dir = interopDir(t)
	stdout, err := srv.StdoutPipe()
	if err != nil {
		t.Fatal(err)
	}
	srv.Stderr = os.Stderr
	if err := srv.Start(); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		_ = srv.Process.Kill()
		_ = srv.Wait()
	})
	// Wait for the "ready" line, so the server is listening before the first call.
	line, err := bufio.NewReader(stdout).ReadString('\n')
	if err != nil || strings.TrimSpace(line) != "ready" {
		t.Fatalf("grpc-core server did not start: %q, %v", line, err)
	}
	waitForPort(t, addr)

	runGoClientChecks(t, addr)
}

// TestInteropGrpcCoreClientToGoServer runs the grpc-core client checks against
// the Go fixture server.
func TestInteropGrpcCoreClientToGoServer(t *testing.T) {
	py := pythonInterpreter(t)
	s := fixtureServer(t)
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	go func() { _ = s.Serve(lis) }()
	t.Cleanup(s.Stop)

	cmd := exec.Command(py, "client.py", lis.Addr().String())
	cmd.Dir = interopDir(t)
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("grpc-core client failed: %v\n%s", err, out)
	}
	if !strings.Contains(string(out), "all interop checks passed") {
		t.Fatalf("unexpected output:\n%s", out)
	}
}

// runGoClientChecks makes the same calls as interop/client.py, against any
// server that serves the test.Echo methods.
func runGoClientChecks(t *testing.T, addr string) {
	t.Helper()
	cc := Dial(addr)
	t.Cleanup(func() { cc.Close() })
	ctx := context.Background()

	var out wrapperspb.StringValue
	if err := cc.Invoke(ctx, "/test.Echo/Unary", wrapperspb.String("hi"), &out); err != nil {
		t.Fatalf("unary: %v", err)
	}
	if out.GetValue() != "echo:hi" {
		t.Errorf("unary = %q", out.GetValue())
	}

	err := cc.Invoke(ctx, "/test.Echo/Fail", wrapperspb.String("héllo 100%"), &out)
	if code, msg := FromError(err); code != NotFound || msg != "no such thing: héllo 100%" {
		t.Errorf("error status = %d %q (%v)", code, msg, err)
	}

	err = cc.Invoke(ctx, "/test.Echo/Nope", wrapperspb.String("x"), &out)
	if code, _ := FromError(err); code != Unimplemented {
		t.Errorf("unknown method = %d (%v)", code, err)
	}

	mdCtx := NewOutgoingContext(ctx, NewMD("x-user", "ann", "x-blob-bin", "\x01\x02"))
	if err := cc.Invoke(mdCtx, "/test.Echo/Meta", wrapperspb.String("x"), &out); err != nil {
		t.Fatalf("metadata: %v", err)
	}
	if out.GetValue() != "ann|\x01\x02" {
		t.Errorf("metadata = %q", out.GetValue())
	}

	dlCtx, cancel := context.WithTimeout(ctx, 200*time.Millisecond)
	defer cancel()
	err = cc.Invoke(dlCtx, "/test.Echo/Slow", wrapperspb.String("x"), &out)
	if code, _ := FromError(err); code != DeadlineExceeded {
		t.Errorf("deadline = %d (%v)", code, err)
	}

	cs, err := cc.NewStream(ctx, "/test.Echo/Count")
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
			t.Fatalf("server streaming: %v", err)
		}
		got = append(got, v.GetValue())
	}
	if fmt.Sprint(got) != "[1 2 3 4 5]" {
		t.Errorf("server streaming = %v", got)
	}

	cs, err = cc.NewStream(ctx, "/test.Echo/Sum")
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
	if err := cs.RecvMsg(&total); err != nil || total.GetValue() != 55 {
		t.Errorf("client streaming = %d, %v", total.GetValue(), err)
	}

	cs, err = cc.NewStream(ctx, "/test.Echo/Upper")
	if err != nil {
		t.Fatal(err)
	}
	words := []string{"alpha", "beta", "gamma"}
	var upper []string
	for _, w := range words {
		if err := cs.SendMsg(wrapperspb.String(w)); err != nil {
			t.Fatal(err)
		}
		var o wrapperspb.StringValue
		if err := cs.RecvMsg(&o); err != nil {
			t.Fatalf("bidi: %v", err)
		}
		upper = append(upper, o.GetValue())
	}
	if err := cs.CloseSend(); err != nil {
		t.Fatal(err)
	}
	if fmt.Sprint(upper) != "[ALPHA BETA GAMMA]" {
		t.Errorf("bidi = %v", upper)
	}
}
