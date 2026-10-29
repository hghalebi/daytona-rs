## Expected result and explanation

```text
Handled exit code 7; sandbox deleted
```

`checked_output` requires a reported zero code. A nonzero code returns
[`ExecutionError::NonZeroExit`](crate::ExecutionError::NonZeroExit) with captured
output. The example recognizes code 7, then returns owned output to the scope.
Cleanup still happens before main prints the result.

| Result | Meaning | Next step |
| --- | --- | --- |
| `CheckedOutput` | Explicit zero exit | Use the owned text |
| `NonZeroExit` | Explicit unsuccessful exit | Inspect the retained code and text |
| `MissingExitStatus` | Provider omitted the status | Keep the result unknown |
| `Operation` | Transport, protocol, or access failure | Inspect the cause and remote uncertainty |

Use [`Shell::output`](crate::Shell::output) when all exit codes are ordinary data
for your application. Then inspect the optional code yourself. Never treat `None`
as zero. Debug redacts text; `text()` exposes it deliberately.

Try changing `exit 7` to `exit 9`. The program returns an execution failure after
cleanup. If the HTTP request times out, the command may still be running; avoid
blind replay. Next: [transfer files](crate::guide::files).
