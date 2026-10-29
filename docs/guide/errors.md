# Handle work and cleanup separately

A command can succeed while deletion fails. A command can fail while deletion
succeeds. Keep those outcomes separate when choosing the next action.

| Scope result | What happened | What you retain |
| --- | --- | --- |
| Success | Work and cleanup succeeded | Owned output |
| `ScopeError::Setup` | Work never began | Setup cause and any cleanup failure |
| `ScopeError::Work` | Work failed; cleanup confirmed | Work error |
| `ScopeError::Cleanup` | Work succeeded; cleanup unconfirmed | Cleanup identity and evidence |
| `ScopeError::Both` | Work and cleanup failed | Both errors |

Use `work_error()` and `cleanup_error()` to inspect both branches. The standard
`source()` chain follows one error chain. Do not rely on it to reveal every failure.

## Keep successful work

`run` returns an error and discards output when cleanup fails. `run_report` keeps
that output in [`ScopeReport::CleanupFailed`](crate::ScopeReport::CleanupFailed).
See the complete [preserve-work handler](crate::guide::errors::preserve_work).

## Recover cleanup

A cleanup error retains the sandbox ID and whether the provider acknowledged
DELETE. Acknowledgement does not confirm removal. Save that information and inspect
the resource using the same provider/account context. Choose any further deletion
attempt explicitly. The SDK does not retry mutations automatically.

Setup errors after uncertain creation require [reconciliation](crate::guide::recovery),
not an assumption that no resource exists. Cancellation never runs async Drop
cleanup. A TTL remains a provider fallback rather than observed evidence.

## Inspect command output

ExecutionError::output borrows retained output from either exit-related failure.
It returns None for transport/protocol failures. Default diagnostics stay redacted.
The [complete diagnostic example](crate::guide::errors::diagnose_failure) shows
both scope error branches and an explicit `--show-output` option.

CleanupError::into_parts moves the identity, acceptance, and reason into recovery
code. Keep the reason as well as the ID; a deadline and an operation rejection
require different decisions.

## Classify failures without parsing messages

Upload and download return TransferError::Limit for the configured file cap.
DELETE 401/403 return provider authentication/authorization errors; other 4xx
except 404 return a provider status rejection. A 404 confirms absence. Server
errors retain protocol status and uncertain deletion acceptance. See the
[0.2 migration guide](https://github.com/hghalebi/daytona-rs/blob/main/docs/migration-0.2.md)
for changed matches. There are no automatic mutation retries.
