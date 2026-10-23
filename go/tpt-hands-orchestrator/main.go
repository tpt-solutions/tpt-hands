// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

// Command tpt-hands-orchestrator is the thin Go service that bridges tpt-swarm
// and the tpt-hands SDK. Routing, device registry and the Cerebrum-facing
// intent path are added in Phase 2 (see todo.md).
package main

import (
	"fmt"
	"os"

	"github.com/tpt-solutions/tpt-hands/go/tpt-hands-orchestrator/internal/version"
)

func main() {
	if _, err := fmt.Fprintf(os.Stdout, "tpt-hands-orchestrator %s\n", version.Version); err != nil {
		os.Exit(1)
	}
}
