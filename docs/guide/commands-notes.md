## Expected result and explanation

```text
Handled exit code 7; sandbox deleted
```

This example uses `output()` because exit code 7 is expected data. It checks the
optional code after cleanup. Await sandbox.run(script) directly when the command must report
zero; a nonzero or missing code then becomes an error retaining the output.

| Result | Meaning | Next step |
| --- | --- | --- |
| `CheckedOutput` | Explicit zero exit | Use the owned text |
| `NonZeroExit` | Explicit unsuccessful exit | Inspect the retained code and text |
| `MissingExitStatus` | Provider omitted the status | Keep the result unknown |
| `Operation` | Transport, protocol, or access failure | Inspect the cause and remote uncertainty |

Use [`Command::output`](crate::Command::output) when all exit codes are ordinary data
for your application. Then inspect the optional code yourself. Never treat `None`
as zero. Debug redacts text; `text()` exposes it deliberately.

Try changing `exit 7` to `exit 9`. The program reports an unexpected exit after
cleanup. If the HTTP request times out, the command may still be running; avoid
blind replay. Next: [transfer files](crate::guide::files).

For a failed checked command, follow [deliberate output inspection](crate::guide::errors::diagnose_failure).
