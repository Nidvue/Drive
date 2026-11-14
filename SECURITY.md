# Security policy

## Reporting a vulnerability

Report privately through GitHub Security Advisories:
<https://github.com/Nidvue/nidvue-drive/security/advisories/new>

Do **not** open a public issue for a vulnerability. Do not paste tokens,
Personal Login Tokens, `credentials.enc`, `credentials.key`, refresh tokens or
full log files into an issue or a pull request. Redact paths and account names.

We aim to acknowledge within 72 hours and to ship a fix or a mitigation plan
within 30 days for high-severity findings.

## What counts as a vulnerability here

- token or credential disclosure through logs, crash reports, telemetry, the
  cache, the journal, error messages, or an unencrypted file;
- a path traversal that escapes a sync root or the mountpoint;
- a plugin escaping its declared capabilities, or a plugin trap that aborts the
  engine;
- silent data loss: a file overwritten, truncated, or deleted without a journal
  record and a conflict copy;
- cache poisoning: a blob served whose hash does not match its name;
- remote code execution, or a panic across the FFI boundary.

## Supported versions

`0.1.x` receives security fixes. Pre-release builds are not supported.

## Design notes for researchers

- Tokens live in `credentials.enc` (ChaCha20-Poly1305) with a `0600` key file,
  or in `NIDVUE_SECRET_KEY` if set. The threat model is stated in
  `crates/nidvue-core/src/secret.rs`.
- The mount never runs setuid; it uses the kernel's unprivileged FUSE path.
- WASM plugins run with fuel, memory and wall-clock limits and no ambient
  authority. Native plugins require an explicit trusted flag and must be
  AGPL-3.0.
