## Expected result

```text
Run directory: .plan/pi-<operation-id>
Program output: 30
Saved: .plan/pi-<operation-id>/sum_even.py
Sandbox deletion confirmed.
```

The generated source can vary. A successful run requires all of these outcomes:

1. Pi creates `sum_even.py` through its write tool.
2. A separate command runs that file and receives exit code zero.
3. Its output is exactly `30` followed by a newline.
4. The SDK downloads the source before deletion.
5. The scope confirms deletion, then a separate observation checks again.

The `30` check proves this one input. It does not prove that every implementation
of `sum_even` is correct for every input. Review the saved code before reusing it.

## Follow the ownership

The client owns its transport. The scope owns the sandbox. The callback borrows
both the sandbox and the local credential path. The downloaded file stays on your
machine after the sandbox is gone. The callback returns `()` because the artifact
has already been saved.

The outer client deadline is 270 seconds. Each long command has a 240-second
provider limit and a 240-second per-request HTTP timeout. Its child process has
a 210-second limit. Raising only the client cap does not increase the per-request
timeout. Network timeout does not prove that remote work stopped.

Pi runs once with `--print`. `--offline` disables automatic catalog refreshes;
the selected model request still uses the network. Only the write tool is enabled.
Extensions, MCP, skills, context files, and session persistence are disabled.
These settings reduce the work allowed in this lesson. They are not a security
boundary for arbitrary model output. See Pi's [CLI reference](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/cli.md).

The model credential lives outside the work directory. An exit handler attempts removal on ordinary success and failure, before the
generated program runs. Its bounded status record retains both work and removal
status. Missing status means removal is unknown. Scope deletion is still attempted
after a returned work error. A killed process, transport loss, or cancelled scope
can prevent these steps. If deletion remains unconfirmed, recover the sandbox and
revoke the exported credential through its provider when needed. TTL is not proof
of erasure. The local credential export belongs to the setup shell;
the Rust example does not modify your original Pi login.

## When something fails

| Outcome | What to do |
| --- | --- |
| Missing `--run-live` | Add it only when you intend to use both accounts |
| Invalid `PI_AUTH_FILE` | Export only the `openai-codex` entry as shown above |
| Login or model rejected | Check your Pi login and model access; the SDK still attempts cleanup |
| npm or remote command fails | Inspect `ScopeError::work_error()`; shell output is redacted in diagnostics |
| Downloaded output is needed after cleanup failure | Look in the printed run directory; downloaded files remain there |
| Cleanup fails | Inspect `ScopeError::cleanup_error()` and the retained sandbox identity |
| Process is stopped or the future is dropped | Reconcile the saved `creation-reference.json`; async cleanup did not run |

Never replay an uncertain creation blindly. Use the
[recovery lesson](crate::guide::recovery). A TTL is a provider fallback, not proof
of deletion. This example makes no automatic retry of model or mutation requests
in SDK code; Pi and provider libraries may have their own retry behavior.

## Try one small change

Ask Pi to create a program that prints the sum of odd numbers from 1 through 10.
Change the independent output check from `30\n` to `25\n`. Keep the same creation,
download, and deletion flow. Keep the prompt fixed while learning; do not insert
untrusted text directly into shell source.

Next: [preserve work when cleanup fails](crate::guide::errors::preserve_work).
