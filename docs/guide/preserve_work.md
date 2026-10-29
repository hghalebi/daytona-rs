# Keep output when cleanup fails

Handle a work report without discarding useful output or cleanup failures.

## Before you start

Read [scope errors](crate::guide::errors). This is a complete handler example. It compiles
without forcing a remote deletion failure. Pass a real `run_report` result to
`handle` in your application.

## Run from the source checkout

```sh
cargo run --example preserve_work
```

The [complete source](https://docs.rs/crate/daytona-rs/0.1.2/source/examples/preserve_work.rs)
is shipped with the crate and included below.

## Complete example
