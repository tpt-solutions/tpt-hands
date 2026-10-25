<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 TPT Solutions -->

# tpt-hands orchestrator

The Go service that bridges tpt-swarm and the tpt-hands SDK. It is thin.

## What it does today

- `GET /healthz` returns `{"status":"ok","version":"..."}`.
- `GET /version` returns `{"version":"..."}`.
- Structured JSON logs on stdout.
- Graceful shutdown on SIGINT or SIGTERM.
- Generated protobuf types for the tpt-hands wire messages (`internal/gen`).
- The Rust interop test for the Ack method (`internal/interop`).

It does not yet route by `tool_id`, keep a device registry, or take intents from
tpt-cerebrum. Those are todo.md, Phase 2.

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `TPT_HANDS_LISTEN_ADDR` | `127.0.0.1:8080` | Address for the health and version endpoints. |
| `TPT_HANDS_LOG_LEVEL` | `info` | One of `debug`, `info`, `warn`, `error`. |

A bad value stops the service at startup, with a message.

## Run it

From this directory:

```
go run .
curl http://127.0.0.1:8080/healthz
```

Or, from the repository root, with Docker. This is not yet built or run. Check
it before relying on it:

```
docker build -f Dockerfile.orchestrator -t tpt-hands-orchestrator .
docker run --rm -p 8080:8080 tpt-hands-orchestrator
```

## Test it

```
go test ./...
```

The interop test builds the Rust `ack-server` and `ack-client` binaries with
cargo, so it needs cargo on the path. It skips if cargo is missing.
