# Changelog

## 0.1.0 — prepared, not yet published

Initial release of the unofficial Daytona Rust SDK. Requires Rust 1.98.0 and a
Tokio runtime. The Cargo package is `daytona-rs`; the library import is `daytona`.

### Included

- Validated identities, credentials, scripts, durations, resources, paths, and
  network policy values with checked deserialization.
- Consuming draft/validated creation states, acknowledged receipts, and explicit
  reconciliation for uncertain submissions.
- A concrete HTTP client with borrowed sandbox handles and scoped async work.
- Separate work, setup, and cleanup failures; successful output can be retained
  when cleanup fails.
- Shell execution that requires an explicit zero exit status for checked success.
- Bounded file upload/download and confirmed sandbox deletion.
- An explicit opt-in live smoke executable with saved recovery references.
- Package inclusion restricted to source/document extensions, with a CI fixture
  ensuring environment files are excluded even inside source directories.
- Bounded concurrency, response capture, deadlines, and read-only polling;
  no automatic mutation retries or redirects.
- Redacted diagnostics, typed errors, compiler misuse tests, local protocol tests,
  and an unpublished source-policy development tool.

### Operational scope

- Verification uses a local HTTP server and deterministic clock tests. An
  authorized live smoke test on 2026-10-29 also passed sandbox creation, shell
  execution, nonzero exit handling, file upload/download, scoped cleanup, and a
  separate provider readback confirming absence.
- Cancellation does not perform asynchronous cleanup. Save the creation/scope
  reference before execution when explicit recovery is required; provider TTL is
  a fallback, not locally verified destruction.
- This release does not implement every Daytona endpoint. Sessions management,
  preview discovery, snapshot management, volumes, and computer-use operations
  are outside its implemented API.
- Automatic platform checks target Linux and macOS; current results are linked
  from the public repository Actions tab.
