# Daytona Rust SDK

[Source](https://github.com/hghalebi/daytona-rs) ·
[Issues](https://github.com/hghalebi/daytona-rs/issues) ·
[Contributing](https://github.com/hghalebi/daytona-rs/blob/main/CONTRIBUTING.md)

An unofficial, independently maintained SDK for Daytona. This project is not
affiliated with or endorsed by Daytona. Version 0.1.0 is the first release candidate;
its public API may change in subsequent 0.x releases.

A Rust SDK with validated domain inputs, consuming creation states, borrowed
sandbox handles, structured recovery errors, and scoped cleanup. The library
crate is named `daytona`; the Cargo package is `daytona-rs`.

This repository implements sandbox creation, readiness observation, creation
reconciliation, shell execution, bounded upload/download, and confirmed deletion.
It is not a generated binding to every Daytona endpoint. Session/command reference
and preview credential types are available as domain values; session management,
preview endpoint discovery, snapshots management, volumes, and computer-use APIs
are not implemented.

## Use

The pinned, tested toolchain and declared minimum Rust version are **1.98.0**.
Async closure support alone does not establish the dependency MSRV.

```toml
[dependencies]
daytona = { package = "daytona-rs", version = "0.1" }
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The registry dependency above becomes available when 0.1.0 is published. For local
work before publication, replace `version = "0.1"` with `path = "../daytona-rs"`.
[API documentation](https://docs.rs/daytona-rs/latest/daytona/) will be available
after docs.rs builds the first release. Use the dependency alias `daytona` as shown
above to match the library import name.

The complete compiled example is [`examples/scoped.rs`](https://github.com/hghalebi/daytona-rs/blob/main/examples/scoped.rs).
It exposes `run_example(&Daytona)`; its `main` does not read credentials or create
billable resources. Applications construct a client explicitly with
`Daytona::new(Credentials::ApiKey(ApiKey::new(key)?))` or `Credentials::Jwt`.
Credentials are never read implicitly from the environment.

```rust,no_run
use daytona::*;

#[derive(Debug, thiserror::Error)]
enum ExampleError {
    #[error(transparent)] Validation(#[from] ValidationError),
    #[error(transparent)] Scope(#[from] ScopeError<ExecutionError>),
}

async fn run(client: &Daytona) -> Result<CheckedOutput, ExampleError> {
    let script = ShellScript::new("printf '%s\\n' 'Hello from Daytona'")?;
    let limit = ExecutionTimeout::seconds(30)?;
    Ok(client.sandboxes().ephemeral()
        .ttl(SandboxTtl::minutes(10)?)
        .run(async move |sandbox| {
            sandbox.shell(script).execution_timeout(limit).checked_output().await
        }).await?)
}
```

`checked_output()` requires an explicit zero exit code. An omitted exit code is
`ExecutionError::MissingExitStatus`, retaining the captured output. Nonzero exits
are `ExecutionError::NonZeroExit`. Both outputs redact `Debug`; read their contents
explicitly through `text()`.

## Ownership and recovery

`Daytona` owns the transport. `Sandbox<'client>` owns an identity and borrows that
client. Neither implements `Clone` or `Copy`. Use borrowed futures with
`tokio::join!` and handle both results, or store IDs in long-lived application
state and construct handles at operation boundaries.

Creation has these consuming transitions:

| Transition | Evidence and recovery |
| --- | --- |
| `Create::draft().validate()` | Requires a finite TTL; failure retains the original draft through `into_parts()` |
| `Create<Validated>::submit(&client)` | Returns a receipt only after decoding an acknowledged sandbox identity |
| `CreateError::NotSubmitted` | Retains the validated request when the local boundary proves it was not sent |
| `CreateError::Rejected` | Retains rejected intent and a typed authentication, authorization, or status error |
| `CreateError::OutcomeUnknown` | Retains a reference that only supports reconciliation; no automatic replay |
| `receipt.wait(&client, timeout)` | Returns a timestamped readiness observation; failure retains the receipt |

Save `serde_json::to_string(request.reference())` **before** awaiting submission
when recovery after cancellation or a process restart matters. Scopes likewise
expose `scope.reference()` before `run`/`run_report`. Decoding a `CreationReference`
validates its values but grants no acknowledgement or authorization. Call
`reference.reconcile(&client).await` to query its correlation label. A single
matching result yields a receipt. Empty, multiple, or paginated results remain
`Unresolved`; missing required pagination metadata is a protocol error.
Correlation labels are not provider idempotency keys. An eventually consistent
empty result does not prove a request failed, and reconciliation never POSTs.

Readiness is a historical observation. Every operation still handles remote state
changes, disappearance, revoked permissions, and transport failure. To inspect
current state, call `sandbox.observe()`; unknown state text and absent state are
retained explicitly.

## Scope and cleanup semantics

The scope owns its sandbox and lends `&Sandbox` to an `AsyncFnOnce` callback. An
owned output can escape the scope; the temporary sandbox borrow cannot.

Normal completion attempts deletion after successful work, failed work, and
failed readiness. `ScopeError` separates setup, work, cleanup, and combined
failures. Its accessors expose both errors when work and cleanup fail. Use
`run_report()` to retain successful output alongside a cleanup failure; the
convenience `run()` discards that output when returning `ScopeError::Cleanup`.

Deletion success requires observing HTTP 404 or the provider's `destroyed` state.
A successful DELETE response alone is acknowledgement. Cleanup errors retain the
resource ID and whether deletion was acknowledged or its acceptance is unknown.
The cleanup deadline covers both DELETE submission and confirmation polling.

Dropping a future, cancelling its task, terminating the process, or panicking in
user work does **not** run asynchronous cleanup. Save the scope reference for
explicit reconciliation and recovery. Every creation sends a positive provider
`ttlMinutes` as a fallback; this does not prove the provider actually destroyed
the sandbox. The callback itself has no imposed work deadline. Caller supervision
must account for cancellation and unfinished remote commands.

## Bounds and protocol behavior

`ClientConfig::cloud()` uses the cloud API and toolbox endpoints. For self-hosted
installations, pass two validated `Endpoint`s to `ClientConfig::new`. HTTPS is
required except for loopback HTTP. URLs cannot contain credentials, queries, or
fragments. Toolbox is an explicitly configured credential recipient; a sandbox
response cannot redirect authentication to a different URL. Multi-region or
self-hosted installations must configure their appropriate toolbox endpoint.

| Limit | Default | Configuration |
| --- | --- | --- |
| Concurrent HTTP operations | 16 | `concurrency(ConcurrencyLimit)` |
| Waiting queue | 0; overload returns `Busy` before sending | Deliberate fail-fast policy |
| HTTP deadline including response body | 60 seconds | `request_timeout(RequestTimeout)` |
| JSON/output capture | 4 MiB | `response_limit(ResponseLimit)` |
| File capture | 16 MiB | `transfer_limit(TransferLimit)` |
| Readiness deadline | 60 seconds | Scope `readiness_timeout(WaitTimeout)` |
| Cleanup deadline | 30 seconds | Scope `cleanup_timeout(WaitTimeout)` |
| Observation polling interval | 250 ms | `poll_interval(PollInterval)` |
| Shell execution timeout | 10 seconds | `execution_timeout(ExecutionTimeout)` |

An explicit execution timeout is sent to the provider and also limits the HTTP
request; the client's HTTP deadline may expire first. Neither deadline proves
that remote execution stopped. Automatic HTTP retries and redirect following are
disabled. Polling performs bounded read-only observations; mutations are not
retried. Response capture is bounded even without a Content-Length header.

`SandboxPath` is an absolute normalized POSIX path; it does not prove filesystem
authorization. Local paths are validated for nonemptiness, and existence/permissions
are checked when used. Download uses `create_new` and preserves existing local
files. A write error can leave a partial newly created file and is reported as
`TransferError::PartialWrite`. Upload overwrites its explicit remote destination;
a transport failure may leave that destination changed. File transfers are bounded
in memory rather than streaming arbitrarily large files.

Memory/disk values use the provider's GB fields. Positive integer domain values
support the signed 32-bit range; provider quotas and organization-specific limits
remain runtime checks. `ImageSpec` contains Dockerfile content, sent as
`buildInfo.dockerfileContent`; snapshot selection uses `SnapshotRef`. Source
alternatives are mutually exclusive. Previews are private in creation requests.

## Architecture and verification

- [`src/domain.rs`](https://github.com/hghalebi/daytona-rs/blob/main/src/domain.rs): nominal values and checked serialization ingress.
- [`src/create.rs`](https://github.com/hghalebi/daytona-rs/blob/main/src/create.rs): state-specific payloads, observations, and recovery.
- [`src/transport.rs`](https://github.com/hghalebi/daytona-rs/blob/main/src/transport.rs): protocol DTOs, bounded HTTP, control/clock boundaries.
- [`src/execution.rs`](https://github.com/hghalebi/daytona-rs/blob/main/src/execution.rs): execution and transfer capabilities.
- [`src/scope.rs`](https://github.com/hghalebi/daytona-rs/blob/main/src/scope.rs): lending scope and independent work/cleanup outcomes.
- [`docs/contracts.md`](https://github.com/hghalebi/daytona-rs/blob/main/docs/contracts.md): architectural contract and limits of the proof.
- [`docs/compile-contracts.md`](https://github.com/hghalebi/daytona-rs/blob/main/docs/compile-contracts.md): positive and negative consumer examples.

Category theory informs the ordinary Rust composition here: products group
required values, sums represent alternatives, validation maps boundary inputs into
valid values, and `Result`/async stages preserve explicit effects and failures.
No custom monad framework is needed.

From a source checkout (which includes the unpublished `source-policy` workspace
tool), run the same gates as CI:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo test --doc --locked
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --locked
cargo install --path tools/source-policy --root target/policy --locked
target/policy/bin/source-policy .
```

The compiled source-policy tool visits every owned `.rs` file outside `.git` and
`target`, including tests, examples, and checked-in generated code, and checks Rust snippets
in Markdown. It rejects
explicit cloning/duplication shortcuts, panic/extraction shortcuts, assertions,
unsafe blocks, indexing, unchecked arithmetic, explicitly discarded results,
and unreviewed macros. Negative fixtures test the scanner itself. Clippy supplies
semantic checks for configured method calls and expanded code. See
[`docs/source-policy.md`](https://github.com/hghalebi/daytona-rs/blob/main/docs/source-policy.md) for the policy's proof boundary.

Tests use a local HTTP server and paused Tokio time, with typed test errors and
no assertion macros. They verify actual wire paths and payloads, no automatic
replay, unknown statuses, readiness and cleanup deadlines, cancellation recovery,
redaction, transfer behavior, and concurrency limits. CI is configured for Linux
and macOS. Local verification is not evidence of a live Daytona deployment;
no cloud resource is needed for the default tests.

Protocol adapters were checked against the official
[Daytona API schema](https://www.daytona.io/docs/openapi.json) and
[Toolbox API schema](https://www.daytona.io/docs/toolbox-openapi.json) on 2026-10-29.
The supported routes and schema hashes are recorded in
[`docs/protocol.md`](https://github.com/hghalebi/daytona-rs/blob/main/docs/protocol.md).


## Release preparation

See [the changelog](https://github.com/hghalebi/daytona-rs/blob/main/CHANGELOG.md) for the initial release scope and
[the release guide](https://github.com/hghalebi/daytona-rs/blob/main/docs/releasing.md) for package verification and publication.
The SDK archive includes its tests and compile-time documentation inputs. The
workspace policy tool, GitHub workflows, credentials, and build artifacts are not
part of the published package.

## License

Licensed under either [the MIT license](https://github.com/hghalebi/daytona-rs/blob/main/LICENSE-MIT) or
[the Apache License, Version 2.0](https://github.com/hghalebi/daytona-rs/blob/main/LICENSE-APACHE), at your option.


## Live verification

An authorized cloud smoke test on 2026-10-29 passed creation, checked shell
execution, nonzero exit handling, upload/download, and scoped cleanup. A separate
provider API request confirmed the sandbox was absent after deletion. This is
point-in-time evidence for those operations, not a service availability guarantee.

The [live smoke example](https://github.com/hghalebi/daytona-rs/blob/main/examples/live_smoke.rs)
is opt-in and is never executed by CI. Supply `DAYTONA_API_KEY` through your
process environment. Optional `DAYTONA_API_URL` and `DAYTONA_TOOLBOX_URL` override
the cloud endpoints; the program does not automatically read dotenv files.

```sh
cargo build --release --example live_smoke --locked
mkdir -p target/bin
install -m 755 target/release/examples/live_smoke target/bin/daytona-live-smoke
target/bin/daytona-live-smoke --run-live
```

Running this command creates a potentially billable sandbox with a five-minute
TTL, performs synthetic probes, then deletes it and reads it back. Recovery
references and receipts are written to ignored `.plan/live-smoke-*` directories.
Retain those records if execution is interrupted or cleanup cannot be confirmed.
Environment files and `.plan` records are excluded from both Git and the crate.
