<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 TPT Solutions -->

# Adding a tool (end-effector)

This guide shows how to add a new end-effector to tpt-hands. A tool is anything
the SDK can drive: a gripper, a finger, a simulated hand, or a real device behind
a transport.

The worked example is `crates/tpt-hands-sdk/examples/new_end_effector.rs`. It is a
two-jaw gripper in 112 lines, and it implements one trait.

Run it with:

```
cargo run -p tpt-hands-sdk --example new_end_effector
```

## 1. Implement `EndEffector`

The trait is in `crates/tpt-hands-sdk/src/end_effector.rs`:

| Method | Purpose |
| --- | --- |
| `identity()` | Stable `ToolIdentity`. Used for routing by `tool_id`. |
| `capabilities()` | What the tool can do right now. Changes after a growth event. |
| `command(intent)` | Applies a `ToolIntent`. Returns `ControlError` if it cannot. |
| `next_telemetry()` | Pops the next queued `ToolTelemetry`, or `None`. |
| `physiological_state()` | Current state, for tpt-soma sync. |

The trait is synchronous and has no runtime dependency. Telemetry is a pull
queue: the caller drains it with `next_telemetry()`. Keep the queue bounded.

```rust
use tpt_hands_sdk::{EndEffector, ToolIdentity, ToolCapabilities, ToolIntent,
    ToolTelemetry, PhysiologicalState, ControlError};

struct MyTool {
    identity: ToolIdentity,
    capabilities: ToolCapabilities,
    // your state here
}

impl EndEffector for MyTool {
    fn identity(&self) -> &ToolIdentity { &self.identity }
    fn capabilities(&self) -> &ToolCapabilities { &self.capabilities }
    fn command(&mut self, intent: ToolIntent) -> Result<(), ControlError> { todo!() }
    fn next_telemetry(&mut self) -> Option<ToolTelemetry> { None }
    fn physiological_state(&self) -> PhysiologicalState { todo!() }
}
```

## 2. Declare capabilities honestly

`ToolCapabilities` is what the orchestrator and Cerebrum use to decide what to
ask for. Report the limits the hardware actually has, such as the maximum force.
Do not report a higher limit to get a task accepted. The tool should refuse
what it cannot do, whatever limits the caller sends.

## 3. Reject invalid intents with `ControlError`

Return an error and leave state unchanged when an intent is invalid. Do not clamp
silently. Errors are the signal the host uses to replan.

## 4. Pathways, if the tool has them

If the tool has tendon-like pathways, model them with `PathwayId` and
`PathwayState` (Intact, Severed with healing progress, Hypertrophied). The
`SimulatedHand` in the SDK shows the healing and growth state machines, and the
`reflex::PathwayMonitor` in `tpt-hands-edge` shows the debounced on-device
equivalent.

## 5. Test it

- Unit-test every `command` path, including the error paths.
- Use `ManualClock` for any time-dependent behaviour, so tests are deterministic.
- For firmware-side logic, put the pure logic in `tpt-hands-edge` so it runs on
  the host. Hardware bring-up is separate.

## 6. Register it

The orchestrator routes by `tool_id`. Register the tool with the device registry
once that lands (see `go/tpt-hands-orchestrator`, Phase 2). Until then, drive the
tool directly from Rust.

## Checklist

- [ ] `EndEffector` implemented, with no unused methods left as `todo!()`
- [ ] Capabilities match the real hardware
- [ ] Invalid intents return `ControlError` and change nothing
- [ ] Command and error paths have unit tests
- [ ] `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt --check` pass
