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

## Documentation and examples

Use short, concrete sentences and consistent domain terms. Document the purpose
first, then the inputs, ownership, effects, errors, and cancellation behavior that
matter. Link to a complete task example rather than repeating the same program.
The crate denies missing public documentation and broken intra-doc links.

The guide includes source directly from `examples/`. Update that canonical file
when changing a lesson. Give each lesson one new idea, expected output, a small
exercise, and a next step. Local lessons run in CI; remote lessons require explicit
execution and credentials. Keep compiled examples distinct from live-test proof.

Run local examples with:

```sh
cargo run --example validate_inputs --locked
DAYTONA_API_KEY=documentation-example cargo run --example configure_client --locked
cargo run --example preserve_work --locked
```

The source-policy tool checks Rustdoc comments as well as Markdown and source
files. Hidden lines follow the same rules. Use `no_run` for remote examples and
`text` for pseudocode; do not use `ignore` or panicking examples to bypass checks.
Review rendered Rustdoc after editing navigation or example layout.
Verification uses Cargo, Clippy, the source-policy tool, and GitHub Actions.
SonarCloud is not used.
