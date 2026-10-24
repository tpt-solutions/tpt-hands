// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

// Command tpt-hands-orchestrator is the thin Go service that bridges tpt-swarm
// and the tpt-hands SDK. It currently serves health and version endpoints.
// Routing, device registry and the Cerebrum-facing intent path are added in
// Phase 2 (see todo.md).
package main

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/tpt-solutions/tpt-hands/go/tpt-hands-orchestrator/internal/config"
	"github.com/tpt-solutions/tpt-hands/go/tpt-hands-orchestrator/internal/health"
	"github.com/tpt-solutions/tpt-hands/go/tpt-hands-orchestrator/internal/version"
)

func main() {
	if err := run(); err != nil {
		fmt.Fprintf(os.Stderr, "tpt-hands-orchestrator: %v\n", err)
		os.Exit(1)
	}
}

func run() error {
	cfg, err := config.FromOS()
	if err != nil {
		return err
	}

	log := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: cfg.LogLevel}))
	log.Info("starting", "version", version.Version, "listen", cfg.ListenAddr)

	srv := &http.Server{
		Addr:              cfg.ListenAddr,
		Handler:           health.NewMux(log),
		ReadHeaderTimeout: 5 * time.Second,
	}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	errCh := make(chan error, 1)
	go func() { errCh <- srv.ListenAndServe() }()

	select {
	case err := <-errCh:
		if errors.Is(err, http.ErrServerClosed) {
			return nil
		}
		return err
	case <-ctx.Done():
		log.Info("shutting down")
		shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		return srv.Shutdown(shutdownCtx)
	}
}
