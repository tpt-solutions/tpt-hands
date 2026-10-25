<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2026 TPT Solutions -->

# Protobuf compatibility rules

These rules govern changes to `proto/tpt_hands.proto`. The schema is still
**provisional** (see the reconcile item in `todo.md`), so the rules are in force
now, and the promise to peers starts at the v1.0 baseline.

## Baseline and check

- `proto/baseline/tpt_hands.proto` is the snapshot that later changes are
  compared with.
- `tools/proto-compat/proto_compat.py` reads both files and fails on any breaking
  change. It runs in CI (`proto` job) and as `just proto-compat`.
- A change that the rules allow is still a change to the baseline. Update the
  baseline in the same commit, and say why in the message.

## What is breaking

A change is breaking if an existing peer built against the old schema could
misread or fail on a message. The checker catches these:

| Change | Why it breaks |
| --- | --- |
| Removing a message or enum | Peers may still send or expect it. |
| Removing a field | The number is then free to reuse with a different meaning. |
| Renumbering a field or enum value | Wire data no longer matches. |
| Changing a field's type | Old bytes decode to the wrong value. |
| Renaming a field | Generated names change, and JSON names change. |

## What is allowed

- Adding a message, enum, or field under a new number. Old peers ignore it.
- Adding a value to an enum. Peers must treat an unknown value as "not recognised",
  not as an error. This is a rule for peer code, not a check.
- Adding a field to a `oneof`.

## Removing things, properly

When a field is no longer used, do not delete the number from the file without
a plan. The checker does not yet support `reserved`, so it refuses any file that
uses one. The steps are:

1. Stop writing the field, and keep reading it while old peers exist.
2. Add `reserved N;` and the old name to the message, after the checker
   supports `reserved`.
3. Only then remove the field from the baseline, in a major version.

## Limits of the check

- It compares structure only. It cannot see changes of meaning, such as a unit
  change or a new default.
- It refuses constructs it does not read: `map<>`, `reserved`, `extend`,
  `service`, and `rpc`. Extending the parser means extending the tests first.
- The two candidate core fields, `request_id` and `command_id`, are not in the
  shared tpt-link core. If the core maintainer moves them, the baseline and the
  generated code both change, and that needs a decision.
- It does not check the shared frame format. That is owned by tpt-link, and its
  compatibility is tpt-link's to police.
