## Expected result and explanation

```text
Hello from Daytona
```

A successful return means the command reported exit code zero and scope cleanup
was confirmed. The output can be printed after the sandbox is gone because it
owns its text.

1. `ApiKey::new` validates the credential text. The provider checks access later.
2. `Daytona::new` owns the transport. The scope borrows the client.
3. The script and execution timeout move into the callback once. Build them first
   so local validation fails before creation. async move transfers these captured
   values; its sandbox argument remains borrowed. The returned execution Result
   tells Rust the callback error type, so no closure type annotation is needed.
4. `checked_output().await` waits for an explicit successful exit.
5. The scope deletes its sandbox after the callback returns, including when work fails.
6. `write!` reports terminal failures through `?` rather than ignoring them.

Try changing the message to your name or project. The output changes; the ownership
and cleanup flow stays the same. Do not put untrusted input directly into shell
source. Treat shell construction as part of your application’s input boundary.

Missing `--run-live` returns a consent error before credentials or network work.
Authentication errors require a valid key. For cleanup errors, inspect the retained
sandbox identity; do not assume it was deleted. See [errors](crate::guide::errors).

## Before adding cancellation

Normal completion runs cleanup. Dropping this future, stopping the process, or
panicking inside user work does not. This first example relies on provider TTL as
a fallback for interruption. Save `scope.reference()` before awaiting when your
application needs restart recovery. See [recovery](crate::guide::recovery).

Next: [inspect command results](crate::guide::commands).
