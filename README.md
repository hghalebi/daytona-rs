# Daytona Rust SDK

Run commands and transfer files in Daytona sandboxes from Rust.

[Get started](https://docs.rs/daytona-rs/latest/daytona/guide/getting_started/) ·
[Learning guide](https://docs.rs/daytona-rs/latest/daytona/guide/) ·
[API reference](https://docs.rs/daytona-rs/latest/daytona/) ·
[Examples](https://github.com/hghalebi/daytona-rs/tree/main/examples) ·
[Crates.io](https://crates.io/crates/daytona-rs)

This is an unofficial, independently maintained SDK. It is not affiliated with or
endorsed by Daytona. The public API may change in subsequent 0.x releases.

## Install

Requires **Rust 1.98.0** and a Tokio runtime.

```toml
[dependencies]
daytona = { package = "daytona-rs", version = "0.1.2" }
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The package is `daytona-rs`. The dependency alias and Rust import are `daytona`.

## Run your first command

The [first command tutorial](https://docs.rs/daytona-rs/latest/daytona/guide/getting_started/)
contains a complete program. It creates a temporary sandbox, prints a message,
and confirms deletion. Credentials come from your application’s environment;
the SDK does not read `.env` files automatically.

From a source checkout, export `DAYTONA_API_KEY` through your secret manager or an
interactive prompt. Then run:

```sh
cargo run --example first_command -- --run-live
```

This explicitly creates a potentially billable resource with a five-minute TTL.
Expected output:

```text
Hello from Daytona
```

The [canonical source](examples/first_command.rs) includes imports, typed errors,
runtime setup, and cleanup. The same file appears in the tutorial and is compiled
in CI. To learn without a key, start with `cargo run --example validate_inputs`.

## Choose your next task

| Task | Guide |
| --- | --- |
| Validate inputs | [Input lesson](https://docs.rs/daytona-rs/latest/daytona/guide/inputs/) |
| Configure limits | [Configuration](https://docs.rs/daytona-rs/latest/daytona/guide/configuration/) |
| Inspect command failures | [Command results](https://docs.rs/daytona-rs/latest/daytona/guide/commands/) |
| Generate code with Pi, run it, and delete the sandbox | [Coding agent](https://docs.rs/daytona-rs/latest/daytona/guide/pi_task/) |
| Upload or download | [Files](https://docs.rs/daytona-rs/latest/daytona/guide/files/) |
| Keep output after cleanup failure | [Error handling](https://docs.rs/daytona-rs/latest/daytona/guide/errors/) |
| Manage a sandbox explicitly | [Lifecycle](https://docs.rs/daytona-rs/latest/daytona/guide/lifecycle/) |
| Recover uncertain creation | [Recovery](https://docs.rs/daytona-rs/latest/daytona/guide/recovery/) |
| Run concurrent commands | [Ownership](https://docs.rs/daytona-rs/latest/daytona/guide/ownership/) |

## What the SDK guarantees

Validated types distinguish IDs, paths, durations, and policies. Creation consumes
local states in order. Sandbox handles borrow the client. Scope results distinguish
work failure from cleanup failure, including when both fail.

A checked command needs an explicit zero exit code. A timeout does not prove that
remote work stopped. Dropping a future does not perform asynchronous cleanup.
Save recovery references before cancellable creation. Provider TTL is a fallback;
confirmed deletion requires an observation. Mutations are not automatically retried.

The SDK supports creation, readiness, reconciliation, shell execution, bounded file
transfer, and confirmed deletion. Session management, preview discovery, snapshot
management, volumes, and computer-use operations are outside the current API.

## Contribute and verify

See [contribution instructions](CONTRIBUTING.md), [contracts](docs/contracts.md),
[source policy](docs/source-policy.md), and the [release guide](docs/releasing.md).
CI checks Linux and macOS, including examples, doctests, compile-fail contracts,
strict Clippy, source policy, and the packaged crate.

The opt-in [live smoke test](examples/live_smoke.rs) also checks cloud execution,
a nonzero exit, a file round trip, and confirmed deletion. Ordinary tests never
create cloud resources. Recovery records belong in ignored `.plan/`; credentials
and those records are excluded from Git and the crate archive.

Report bugs through [GitHub issues](https://github.com/hghalebi/daytona-rs/issues).
Use [private vulnerability reporting](SECURITY.md) for security issues.
See the [changelog](CHANGELOG.md) for release details.

Licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
