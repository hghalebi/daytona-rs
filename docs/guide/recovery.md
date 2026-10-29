# Reconcile uncertain creation

Look up a saved creation reference without creating another sandbox.

## Before you start

Read [manual lifecycle](crate::guide::lifecycle). Use the same API/account context that
created the reference. This example only reads remote state; it does not delete or
recreate a sandbox. It needs a JSON reference saved before creation submission.

Replace the example path below with your saved reference. The operation ID in the
filename varies. The live smoke example stores a reference in its receipt directory.

## Run from the source checkout

```sh
cargo run --example recover_creation -- .plan/manual-YOUR_OPERATION_ID.json
```

The [complete source](https://docs.rs/crate/daytona-rs/0.1.2/source/examples/recover_creation.rs)
is shipped with the crate and included below.

## Complete example
