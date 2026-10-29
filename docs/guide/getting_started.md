# Run your first command

Print a message in a temporary sandbox, then confirm its deletion.

## The workflow

1. Validate the script, execution limit, and sandbox lifetime locally.
2. Borrow one client and let a scope create the sandbox.
3. Run the command through the borrowed sandbox.
4. Receive owned output after deletion is confirmed.

The [small workflow at the crate root](crate) shows these steps with a configured
client. The complete program below includes all setup and error handling.

## Before you start

You need Rust 1.98.0, a Tokio runtime, and a Daytona API key. Basic Rust `Result`
and async syntax are enough. This example creates a potentially billable sandbox
with a five-minute TTL. Run it only when you intend to create that resource.

In a new application, add:

```toml
[dependencies]
daytona = { package = "daytona-rs", version = "0.2.0" }
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Copy the complete program below into `src/main.rs`. The package name is
`daytona-rs`; the dependency alias and Rust import are `daytona`.

Set `DAYTONA_API_KEY` through your process environment or secret manager. For an
interactive Bash or Zsh terminal, this reads the key without adding it to shell
history or displaying it:

```sh
read -r -s DAYTONA_API_KEY
export DAYTONA_API_KEY
```

Type the key at the prompt, then press Enter. The SDK does not load `.env` files.
Run your new application with `cargo run -- --run-live`.
The checkout command below runs the same canonical example.

## Run from the source checkout

```sh
cargo run --example first_command -- --run-live
```

The [complete source](https://docs.rs/crate/daytona-rs/0.2.0/source/examples/first_command.rs)
is shipped with the crate and included below.

## Complete example
