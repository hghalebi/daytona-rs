# Migrate from 0.1 to 0.2

Historical guide for 0.1 to 0.2. For the current API, use the
[0.3 migration guide](migration-0.3.md).

The common scoped workflow is unchanged. Version 0.2 adds configured scopes,
independent command budgets, and explicit lifecycle admission. Update exhaustive
error matches for the new evidence below.

| Boundary | 0.1 behavior | 0.2 behavior |
| --- | --- | --- |
| DELETE rejected before transmission | Unknown acceptance | NotSubmitted acceptance |
| DELETE 401/403 | Protocol status and Unknown | Provider authentication/authorization rejection and Rejected |
| DELETE other 4xx except 404 | Protocol status and Unknown | Provider status rejection and Rejected |
| DELETE 404 | Confirmed absence | Unchanged |
| DELETE 5xx | Protocol status and Unknown | Unchanged |
| Poll fails after accepted DELETE | Acknowledged | Unchanged; acknowledgement retained |
| Download exceeds transfer cap | Operation/Transport/BodyLimit | TransferError::Limit, same as upload |
| Lifecycle capacity wait expires | No waiting mode | TransportError::CapacityDeadline, request unsent |

DeleteAcceptance now has NotSubmitted and Rejected variants. TransportError now
has CapacityDeadline. Update exhaustive matches. NotSubmitted describes this
attempt only; it is not proof that an earlier attempt had no effect. A rejection
also says nothing about prior attempts. Neither automatically authorizes replay.

NeedsTtl now carries private state. Construct scopes through Daytona::sandboxes;
code that instantiated the old unit marker must use that entry point. No new
proof-bearing state can be deserialized or freely constructed.

Default HTTP admission remains fail-fast. LifecycleCapacity::Wait is opt-in and
shares the same active request limit. Its queue and waiting time are bounded.
The client cap and total readiness/cleanup deadlines still apply.

execution_timeout keeps its behavior. budget(CommandBudget) separates provider
execution from HTTP waiting. The last of these setters wins. There is no automatic
retry and no new promise that timeout stops remote work.

The recovery teaching example now uses a versioned application record with an
explicit cloud environment and organization context. Older bare-reference files
remain valid SDK CreationReference data, but are not records for that example.
The SDK reference wire format is unchanged.

New convenience methods borrow retained output or move evidence intact:
ExecutionError::output, CleanupError::into_parts, and Sandbox::into_id. Debug
remains redacted. A local ID transfer does not stop or delete a remote sandbox.

Creation validation now rejects resources with a default or named snapshot via
ValidationError::ResourcesRequireImage. The retained draft can be corrected by
selecting an image source. This prevents a provider rejection; snapshot sizes
remain defined by the snapshot. Update exhaustive validation-error matches.
