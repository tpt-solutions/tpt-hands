// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

// Package health serves the orchestrator's liveness and version endpoints.
package health

import (
	"encoding/json"
	"log/slog"
	"net/http"

	"github.com/tpt-solutions/tpt-hands/go/tpt-hands-orchestrator/internal/version"
)

// Status is the JSON body returned by the health endpoint.
type Status struct {
	Status  string `json:"status"`
	Version string `json:"version"`
}

// NewMux returns a handler serving GET /healthz and GET /version.
// Other paths return 404, and other methods return 405.
func NewMux(log *slog.Logger) *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /healthz", func(w http.ResponseWriter, _ *http.Request) {
		writeJSON(w, log, Status{Status: "ok", Version: version.Version})
	})
	mux.HandleFunc("GET /version", func(w http.ResponseWriter, _ *http.Request) {
		writeJSON(w, log, map[string]string{"version": version.Version})
	})
	return mux
}

func writeJSON(w http.ResponseWriter, log *slog.Logger, v any) {
	w.Header().Set("Content-Type", "application/json")
	if err := json.NewEncoder(w).Encode(v); err != nil {
		log.Error("encode health response", "err", err)
	}
}
