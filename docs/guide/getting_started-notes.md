## Expected result

```text
Hello from Daytona
```

The complete workflow is client.run(script).await. The SDK validates the script,
creates a temporary sandbox, checks the command exit, and confirms deletion.
The output owns its text and remains usable after deletion.

Defaults are a five-minute provider TTL, a thirty-second command limit,
a sixty-second readiness deadline and a thirty-second cleanup deadline. The
client HTTP cap still applies. These are separate limits, not one total deadline.

The application supplies credentials and a Tokio runtime. The SDK does not read
.env files or start a runtime. The example checks --run-live before credentials
or network work. Its fallible write reports terminal errors through a typed Result.

Try changing the greeting. Treat shell source as code: do not insert untrusted
input without an appropriate application-level encoding strategy. The SDK does
not add quoting, shell flags or interpolation.

## Failure and recovery

RunError distinguishes invalid input, setup, command, cleanup and combined
failures. A cleanup error retains successful output. Inspect successful_output()
and cleanup_error(), or match the error to take ownership of both.

Dropping the future or stopping the process does not perform asynchronous cleanup.
A provider TTL is a fallback, not deletion evidence. For restart recovery, store
the task in a variable, persist task.reference() with the intended environment
and settings, and then await it. See [recovery](crate::guide::recovery).

Next: [macro options](crate::guide::macros) or [command results](crate::guide::commands).
