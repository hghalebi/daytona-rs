# Inspect command results

Handle a deliberate nonzero exit and still complete cleanup.

## Before you start

Complete [your first command](crate::guide::getting_started). Supply the same real API
key. This example creates one temporary sandbox and runs `exit 7`.

## Run from the source checkout

```sh
cargo run --example inspect_exit -- --run-live
```

The [complete source](https://docs.rs/crate/daytona-rs/0.2.0/source/examples/inspect_exit.rs)
is shipped with the crate and included below.

## Complete example
