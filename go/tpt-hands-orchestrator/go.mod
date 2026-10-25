// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

module github.com/tpt-solutions/tpt-hands/go/tpt-hands-orchestrator

go 1.26.0

require (
	github.com/tpt-solutions/tpt-hands/go/tpt-grpc-lite v0.0.0
	google.golang.org/protobuf v1.36.12
)

require (
	golang.org/x/net v0.61.0 // indirect
	golang.org/x/text v0.43.0 // indirect
)

replace github.com/tpt-solutions/tpt-hands/go/tpt-grpc-lite => ../tpt-grpc-lite
