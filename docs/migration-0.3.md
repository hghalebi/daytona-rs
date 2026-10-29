# Migrate from 0.2 to 0.3

Version 0.3 intentionally replaces the convenience API. Domain values and remote
protocol behavior remain explicit. There are no compatibility aliases.

| Previous call or type | Replacement |
| --- | --- |
| sandboxes().ephemeral().ttl(value).run(work) | client.scope(work).ttl(value).await |
| sandboxes().ephemeral_from(request).run(work) | client.scope_from(request, work).await |
| scope.run_report(work).await | client.scope(work).report().await |
| sandbox.shell(script).checked_output().await | sandbox.run(script).await |
| sandbox.shell(script).output().await | sandbox.run(script).output().await |
| sandboxes().create() | Create::draft() |
| ScopeError<E> | ScopeError<T, E>, or RunError for one checked command |
| ScopeError::Cleanup(error) | ScopeError::Cleanup { output, cleanup } |

The old Ephemeral, NeedsTtl, HasTtl, Shell and Sandboxes facade types are removed.
Awaiting client.run(script) handles the complete temporary-sandbox lifecycle.
Awaiting sandbox.run(script) executes in an existing sandbox and never deletes it.
Text and String input are accepted at these boundaries; validation completes before
execution. Existing ShellScript values move into the same API without copying.

The default command execution and request limit is now 30 seconds instead of the
old 10-second provider default. Client HTTP caps still apply. Explicit execution
limits and CommandBudget retain their documented last-setting precedence.
Default temporary TTL is 5 minutes. Scope callbacks have no implicit work deadline.

Preparation errors retain the owned draft and original reference. A corrected
field must be supplied before resuming an invalid setting. New ExecutionError and
ScopeError preparation variants require updates to exhaustive matches.

ScopeReport::into_result now preserves successful output inside cleanup errors.
Use successful_output() to borrow it or match the variant to move it. Error Debug
and report Debug redact arbitrary callback output and work errors. There is no
new Debug or Clone requirement on callback output.

Creation references must be persisted before awaiting when recovery matters.
For a callback that needs a directory named from the reference, first validate a
Create request, persist its reference and prepare the directory, then pass the
request and callback to scope_from. It preserves the exact reference and policies.

See the complete macro guide for run!, scope!, ttl! and execution_timeout!,
including single evaluation, invalid options, dynamic values and cancellation.
All examples in the current learning guide use the 0.3 API.
