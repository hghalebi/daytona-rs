# Give a command time to return its result

Use the first-command setup. This live example creates a billable sandbox and
runs a small command with an explicit budget; it does not spend 30 seconds sleeping.

```sh
cargo run --example long_command -- --run-live
```

| Limit in this example | Value | Meaning |
| --- | --- | --- |
| Provider execution | 30 seconds | Requested remote command limit |
| Command HTTP wait | 40 seconds | Time to receive headers and body |
| Client cap | 60 seconds | Outer transport deadline; still applies |

CommandBudget combines distinct validated values. A shorter HTTP wait is allowed
when early return is intentional; it does not prove the process stopped. The last
budget() or execution_timeout() call wins. The old setter still gives execution
and HTTP the same limit. Without either setter, execution defaults to 10 seconds.

The transport clock starts around request execution and body capture. A provider
starts its own execution clock remotely. These clocks are not synchronized. A
callback deadline bounds your work future; readiness and cleanup have separate
scope deadlines. None authorizes automatic mutation replay.

Output is captured within the configured response limit. Write large results to
a remote file and download before cleanup. Downloads are also buffered and limited;
this is not a streaming API. Increase limits deliberately or split the artifact.

Expected: `Hello from Daytona`, then confirmed cleanup. Exercise: lower the client
cap and explain which limit can now end waiting first. Next: [files](crate::guide::files).

## Complete example
