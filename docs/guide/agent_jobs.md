# Run bounded agent jobs

Own one client in a supervisor. Prepare a small batch, then borrow that client
until all jobs finish. Each job owns its script and returns owned output. This
example uses two shell jobs so you can learn orchestration before adding a model.

Use the first-command dependencies plus serde_json. The live command creates two
billable sandboxes and saves their results under separate .plan/job-* directories:

```sh
cargo run --example agent_jobs -- --run-live
```

Expected: `Both artifacts saved; both sandboxes deleted`.

## Follow the handovers

1. Validate both jobs and create both scopes locally.
2. Persist both creation references before starting either future.
3. Run a fixed batch of two borrowed futures. The client also caps active HTTP
   requests at two; lifecycle waiting has its own bounded pending count.
4. Bound each work callback. A returned work timeout still permits cleanup.
5. Save owned output even when cleanup fails. Preserve both failures if saving
   also fails. Both job results are handled before the supervisor ends.

In a production application, persist the trusted account/endpoint context with
references as shown in the recovery lesson. These example directories hold bare
references and are not the recovery lesson's versioned records. A timeout or
whole-supervisor cancellation still needs explicit reconciliation.

For large artifacts, download inside the callback before deletion; the
[Pi lesson](crate::guide::pi_task) shows this flow. Pi is a harness that performs
model calls; the SDK owns sandbox lifecycle. Keep model credentials separate from
Daytona credentials and pass user tasks through data files, never shell interpolation.

| Pattern | What is verified |
| --- | --- |
| Borrowed concurrent commands | Compiled Tokio join in concurrent_commands |
| Borrowed jobs and owned results | Compiled fixed batch in this example |
| Every possible scope callback returns a Send future | Not promised |
| Detached jobs borrowing a client that may be dropped | Not supported by this lifetime model |

Exercise: make one script fail. Both jobs must still finish their cleanup attempts,
and the other artifact remains saved. Next: [run Pi](crate::guide::pi_task).

## Complete example
