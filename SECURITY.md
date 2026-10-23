# Security policy

## Reporting a vulnerability

Please do not report security issues in public GitHub issues.

Report them privately to the maintainers at TPT Solutions. The security contact
address is still to be confirmed (see the open item in todo.md). Until it is
set, use a private GitHub security advisory on this repository.

Include:

- the affected component (crate, firmware target, or protocol message)
- steps to reproduce, or a proof of concept
- the expected impact

## Scope of particular concern

- The smart power handshake (Ed25519 challenge/response and relay control).
  Relay closure without a valid signature is a critical defect.
- Key handling. Signing keys must never be committed. `.gitignore` excludes
  `*.pem`, `*.key`, `*.secret` and `.env`.
- Frame parsing on the link (length, CRC, sequence and version handling).
