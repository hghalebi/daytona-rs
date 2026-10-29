# Inspect a failed command deliberately

Use the first-command dependencies and a Daytona key. This live lesson creates a
billable temporary sandbox. The command prints a message and exits with code 7.

```sh
cargo run --example diagnose_failure -- --run-live
cargo run --example diagnose_failure -- --run-live --show-output
```

Both runs return a failing process status after attempting cleanup. The first
prints a readable work error. The second also prints `Example failure details`.
Only enable output when it is safe to display: a real command may print secrets.
If work and cleanup both fail, both branches are shown. A setup error means work
never began; consult the recovery lesson before repeating an uncertain create.

Exercise: remove `exit 7`, then compare the result. Next: [preserve useful
work](crate::guide::errors::preserve_work).

## Complete example
