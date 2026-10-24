<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 TPT Solutions -->

# Ecosystem integration: tpt-soma and tpt-chrysalis

Status: findings from reading the public repos, 2026-10-25. Written for the
Phase 2 adapters. It records what the real APIs offer, where DESIGN.md's plan
does not fit them, and the decisions that belong to the repo owners.

Sources read (public, TPT-Solutions on GitHub):

| repo | commit | what was read |
| --- | --- | --- |
| tpt-soma | `40b4aa5` (2026-08-08) | `crates/tpt-soma-api/src/flight.rs`, `schemas/protobuf/*.proto` |
| tpt-chrysalis | `7ca441a` (2026-08-05) | `crates/tpt-chrysalis-api/src/{api,lib,tokens}.rs`, `crates/tpt-chrysalis-hal/src/actuator.rs`, `crates/tpt-chrysalis-safety/src/protocol.rs` |

Not found in either repo: a gRPC service definition, any `MorphogenesisRequest`
or equivalent, any device-telemetry ingest, and any "physiological state" type.

## 0. Status after the 2026-10-25 changes (local master, not pushed)

- **tpt-soma `00004e3`.** Flight `do_put` now accepts the `device_state` subject
  for tpt-hands telemetry. It needs an `ingest` token for resource class
  `device_state`, and `read` tokens are refused. Batches are validated in full,
  and rows go to `device_state_observations` (migration `000009`). The database
  test needs `TEST_DATABASE_URL`, and it has not been run here. Reads and writes
  are still not audit-logged.
- **tpt-chrysalis `014a7ab`.** `POST /api/v1/device-commands` accepts
  `heal_pathway`, `sprout_conduit` and `reinforce_tendon`. Every kind is refused
  with 422 until it has a gate rule and a HAL mapping. Nothing reaches the
  orchestrator. The gate's default-allow for unmatched steps is still there, and
  so is the refusal gap in the audit ledger.
- Sections 1-3 below describe the APIs as they were before these commits.

## 1. What the real APIs are

### tpt-soma: a patient-data platform

tpt-soma is a biomedical data platform. Its `.proto` files describe samples,
patients and cohorts (`schemas/protobuf/sample.proto`). Its API is Arrow
Flight, backed by Postgres.

- Reads work. `do_get` serves these subjects, which are the only ones the
  server maps (`flight.rs:62-66`): `variants`, `expression` and `umap`,
  `clinical_observations`, `cgm`, and `simulation`.
- Writes do not. `do_put` and `do_action` both return
  `Status::unimplemented` (`flight.rs:349-361`).
- There is no inbound path for device data, and no outbound path for repair
  directives.

### tpt-chrysalis: a lab-automation platform with a verified gate

tpt-chrysalis controls organ-on-a-chip and microfluidic lab equipment. Its
model is a Physical-Digital State Graph (PDSG), with a fail-closed safety gate
and capability tokens. Its API crate describes itself as a "Phase 1 service" and says the HTTP
surface is "a dev/loopback surface" (`crates/tpt-chrysalis-api/src/lib.rs:1`,
`src/http.rs:6`).

- The write intent I found is `POST /api/v1/intents`, with body `SubmitIntentRequest`
  (I did not find others, but this search was not exhaustive):
  `id`, `reagent`, `target_well`, `volume_ul`, optional `temperature_milli_c`,
  and `environment` (gate variables such as `pressure_mbar`) (`api.rs:124-137`).
- The device vocabulary is `Dispense`, `SetValve` and `SetTemperature`
  (`tpt-chrysalis-hal/src/actuator.rs:70-76`).
- A missing gate variable fails the gate closed (`api.rs:124-126`).
- Actions need a capability token (`tokens.rs`).

## 2. Mapping against DESIGN.md

### tpt-hands → tpt-soma (`PhysiologicalUpdate`)

| DESIGN.md | tpt-soma | status |
| --- | --- | --- |
| `PhysiologicalUpdate` stream (client streaming) | No ingest. `do_put` is unimplemented. | **gap** |
| Repair directives from soma (Phase 2 item) | No outbound API. | **gap** |
| `simulation` subject | Simulation output for patient models. Not a device twin. | not a match |

The closest existing thing is `do_put`, which is a stub. Adding device-state
ingest there would mean changing tpt-soma.

### tpt-hands → tpt-chrysalis (`MorphogenesisRequest`)

| DESIGN.md | tpt-chrysalis | status |
| --- | --- | --- |
| `HealPathway` | No heal action. Nearest: `Dispense` of a precursor, plus `SetTemperature` for `heat_assist`. | **mapping by analogy** |
| `SproutConduit` | No conduit action. Nearest: `Dispense` of a liquid metal. | **mapping by analogy** |
| `ReinforceTendon` | No tendon action. Nearest: `Dispense` of hydrogel. | **mapping by analogy** |
| `SafetyEnvelope` (kPa, °C, mL) | Gate variables such as `pressure_mbar` and `temperature_milli_c`. | **unit mismatch**: 1 kPa = 10 mbar |
| `MorphogenesisAck` (`chrysalis_job_id`) | Intent `id` in the request. | workable |
| Capability checks | Capability tokens. | workable, needs a token flow |

The analogy holds only in the sense that both systems dispense fluid into
addressed locations. In chrysalis, `target_well` is a lab well. Tendons,
conduits and pathways do not exist there. Using `target_well` for a tendon id
would pass the gate and misrepresent the physical meaning.

## 3. Consequences for Phase 2

1. The integration success criterion in DESIGN.md ("the adapter works against
   the real tpt-chrysalis API") cannot be met as the APIs stand. Either the
   APIs gain the operations, or the criterion changes.
2. The Phase 2 mock servers can still test the adapters' own logic: job
   lifecycle (submitted, approved, running, done, failed), backpressure,
   reconnect and buffering. The mocks must be labelled as mocks of a proposed
   interface, not of the real service.
3. The mapping table above is a proposal. It needs the chrysalis and soma
   maintainers before any adapter depends on it.

## 4. Decisions for the repo owners

These are not for tpt-hands to decide alone:

- **tpt-soma.** Accept device-state telemetry through `do_put` (a new subject,
  for example `device_state`), and decide how repair directives are sent back.
- **tpt-chrysalis.** Either add a generic device-command intent, or add
  morphogenesis action types, and decide the unit for pressure. The gate
  definition belongs to chrysalis, so the choice of gate variables does too.
- **tpt-hands.** Choose between (a) mock-first adapters against a proposed
  interface, and (b) waiting for the owners' answers before building the
  adapters.

## 5. Not covered here

- Authentication. tpt-soma uses Ed25519-signed bearer tokens (`ed25519_dalek`
  in `flight.rs`). tpt-chrysalis uses its own capability tokens. Neither is
  mapped to tpt-hands' Phase 3 handshake.
- Go. tpt-swarm exposes protobuf services in `proto/` (`robot.proto`,
  `fleet.proto`, `skill.proto`, `federated.proto`, package `swarm.v1`). The
  orchestrator bridge is not yet mapped.
