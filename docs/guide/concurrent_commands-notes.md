## Expected result and explanation

```text
first
second
```

The remote operations may complete in either order. Main prints their owned results
in the order shown. `tokio::join!` borrows both futures until both finish. It does not
require cloning the client or moving a borrowed handle into a detached task.

The match retains both failures when both commands fail. Using `first?; second?`
would lose the second failure when the first failed. Scope cleanup runs after the
callback returns.

Try making the second command exit unsuccessfully. The result is a typed work
failure after cleanup. Limits still apply: concurrency does not create an unbounded
waiting queue. Next: [troubleshooting](crate::guide::troubleshooting).
