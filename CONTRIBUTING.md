# Contributing

Issues and pull requests are welcome at
[hghalebi/daytona-rs](https://github.com/hghalebi/daytona-rs). For a substantial
API change, describe the use case and proposed ownership/error contract in an
issue before implementing it. Small fixes can go directly to a pull request.

## Development

Install Rust 1.98.0 with rustfmt and Clippy. Clone the repository and run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo test -p daytona-rs --doc --locked
RUSTDOCFLAGS='-D warnings' cargo doc -p daytona-rs --no-deps --locked
cargo install --path tools/source-policy --root target/policy --locked
target/policy/bin/source-policy .
```

The default tests use local mock servers and require no Daytona credentials.
See [the release guide](docs/releasing.md) for package and publish dry-run checks.
A real cloud smoke test can create billable resources; it must be explicitly
configured and authorized, and must retain a cleanup/recovery path.

## Rust contracts

Read [the implementation contract](docs/contracts.md) and
[the source policy](docs/source-policy.md). In particular:

- Validate domain inputs through private newtypes and checked decoding.
- Use consuming state transitions for local lifecycle guarantees; observe mutable
  remote facts at runtime.
- Borrow or move handles and values. Do not add explicit cloning or Clone/Copy
  implementations to bypass ownership constraints.
- Define SDK-owned errors with thiserror and retain recovery data when an effect
  may have happened. Do not automatically replay uncertain mutations.
- Keep trait bounds minimal and capability boundaries meaningful.
- Avoid panic/extraction shortcuts, runtime assertions, unsafe code, unchecked
  indexing/arithmetic, and discarded failures, including in tests and examples.
- Test new guarantees with both valid use and intended misuse. Keep protocol
  fixtures aligned with linked provider documentation.

Pull requests should explain the observable behavior, compatibility impact,
validation performed, and any remaining runtime limitations. Never include tokens,
real customer output, private endpoint details, or local `.plan` documents.

Contributions are accepted under the repository's existing dual MIT/Apache-2.0
license. Keep discussion respectful and focused on the code and its users.
