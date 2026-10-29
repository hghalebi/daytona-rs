# Manage a sandbox explicitly

Understand the consuming steps behind a scope.

## Before you start

Read [scope errors](crate::guide::errors). This advanced example creates a sandbox.
Supply a real API key and run from a directory where `.plan` can be created.
Keep `.plan` private and excluded from version control in your application.

```text
Draft -> Validated -> CreationReceipt -> ReadyObservation -> Sandbox
                                                        -> confirmed deletion
```

Each arrow changes local knowledge or ownership. Remote state can still change.

## Run from the source checkout

```sh
cargo run --example manual_lifecycle -- --run-live
```

The [complete source](https://docs.rs/crate/daytona-rs/0.1.2/source/examples/manual_lifecycle.rs)
is shipped with the crate and included below.

## Complete example
