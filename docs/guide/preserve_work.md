# Keep output when cleanup fails

Run a local simulation of all four report outcomes. No key is needed.

```sh
cargo run --example preserve_work
```

Expected: `Four report outcomes handled; useful output retained. No cloud requests.`
The simulation uses ordinary strings and explicitly synthetic failures. It does
not manufacture a successful remote execution proof.

For real work, use the first-command dependencies and a Daytona key, then run:

```sh
cargo run --example preserve_work -- --run-live
```

This creates a billable sandbox, runs a command, and attempts deletion. A deadline
inside the callback returns a work error so cleanup can run. Cancelling the whole
scope instead requires recovery from a saved reference.

## Complete example
