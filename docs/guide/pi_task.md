# Run a coding agent, then delete its sandbox

Ask Pi to create a Python program. Run the program, save its source locally, and
confirm that the sandbox is gone.

This lesson builds on [your first command](crate::guide::getting_started) and
[file downloads](crate::guide::files::download). The SDK manages the sandbox.
Pi manages the model conversation and its tool calls.

## Before you start

You need Rust 1.98.0, a Daytona API key, `jq`, and a working Pi login for
`openai-codex` with access to `gpt-6-luna`. This example uses your model account
and creates a potentially billable sandbox with a 15-minute TTL.

The remote sandbox needs Node.js 22.19 or newer, npm, Python 3, and `timeout`.
The tested Daytona default image supplied these commands. Custom images must
supply them too. The example installs Pi **1.4.0** with npm lifecycle scripts
disabled. The Pi version is fixed; npm transitive dependencies are not fully pinned.

Set `DAYTONA_API_KEY` in your process environment. The SDK does not load `.env`
files. For Pi setup, see the upstream [provider guide](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/providers.md).
You can inspect available models with `pi --list-models openai-codex`.

## Run from the source checkout

Use a temporary file containing only the selected Pi credential. Do not upload
your complete Pi profile. This command checks the local login, exports one
credential, runs the example, and removes the local export when the shell exits:

```sh
(
  set -eu
  umask 077
  pi auth check --provider openai-codex
  pi_credentials_dir=$(mktemp -d)
  trap 'rm -f "$pi_credentials_dir/auth.json"; rmdir "$pi_credentials_dir"' EXIT
  jq -e '{"openai-codex": .["openai-codex"]} | select(.["openai-codex"] != null)' \
    "$HOME/.pi/agent/auth.json" > "$pi_credentials_dir/auth.json"
  export PI_AUTH_FILE="$pi_credentials_dir/auth.json"
  cargo run --example pi_task -- --run-live
)
```

The exported credential is sent to the temporary sandbox. Use an account you
authorize for that environment. The Daytona API key stays in the Rust client.
The example rejects auth files containing other provider entries. It never
prints credentials, the model transcript, or remote command output on failure.

For an application outside this checkout, use the dependencies from
[getting started](crate::guide::getting_started) and add `serde_json = "1"` plus `serde = { version = "1", features = ["derive"] }`.
Copy the program below into `src/main.rs`, then use `cargo run -- --run-live`.

The [complete source](https://docs.rs/crate/daytona-rs/0.2.0/source/examples/pi_task.rs)
is shipped with the crate and included below.

## Complete example
