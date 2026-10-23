# Contributing to tpt-hands

Thanks for your interest. Before you start:

1. Check [todo.md](todo.md) and [DESIGN.md](DESIGN.md). Decisions that are
   already locked in are listed in both.
2. For larger changes, open an issue first so the design can be agreed.

## Licensing of contributions

tpt-hands is dual-licensed under MIT OR Apache-2.0. Contributions are accepted
under the same terms by default. By submitting a pull request you agree that
your contribution is licensed under the MIT and Apache-2.0 licences, without
additional terms.

Every new source file needs an SPDX header:

```rust
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions
```

## Dependencies

- Permissive licences only (MIT, BSD, ISC, Zlib, Unicode, or `MIT OR Apache-2.0`).
- No Apache-2.0-only dependencies. In particular, `prost` is banned.
- Run `just deny` before you open a pull request. CI runs the same check.

## Before you open a pull request

```sh
just check
```

This runs formatting, clippy (warnings are errors) and the tests. CI runs them
on Linux and Windows.
