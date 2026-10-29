# Implementation contract

Domain values use private validated representations; JSON ingress invokes the
same constructors. IDs are opaque nonempty ASCII path tokens, not proof of
existence. Paths are absolute normalized POSIX paths without parent traversal.
Memory and disk allocations use provider GB units; all duration units are named.
Credentials, scripts, output, requests, and recovery records redact Debug.

Creation consumes Draft -> Validated -> acknowledged receipt or contextual error.
Validation failure returns its draft. Before transmission failure returns the
validated request; definitive rejection returns rejected intent; ambiguous
responses retain a reconciliation reference and cannot be submitted again.
A UUID label is correlation, not a provider idempotency guarantee. Persist the
borrowed creation reference before awaiting submission when cancellation matters.
Restored references can only reconcile. Empty eventually consistent results do
not authorize replay. No automatic HTTP retries or redirect following.

The concrete client owns its HTTP pool. Handles borrow the client and own IDs.
Scopes own their handles and lend them through AsyncFnOnce. Scope completion
attempts deletion after readiness failure, successful work, and failed work;
accepted deletion and confirmed absence/destruction are distinct. Cancellation,
process termination, and user panic cannot run asynchronous Drop cleanup. A
provider wall-clock TTL is sent on every creation as a fallback, not a locally
verified guarantee of destruction. Callbacks that do not finish require caller
supervision. No universal Send promise is made for scoped callbacks.

Internal control, execution, transfer, and clock traits isolate effects. Bounds
are only imposed where used. Network observations and authorization remain runtime
facts. Responses and file contents are bounded; in-flight network operations have
a configurable limit and a zero-length waiting queue (overload fails immediately).
HTTP operations have deadlines; polling has a total deadline and fixed positive
backoff. Side effects are never retried. Download creates a new local file and
never overwrites an existing destination; partial write failures are explicit.

Evidence: checked ingress/round trips, consumer compile-fail examples paired with
positive examples, local HTTP protocol tests, deterministic clock tests, and an
AST/token source-policy executable with negative fixtures. This policy excludes
explicit cloning and common panic paths in owned sources, not dependency internals,
allocation failure, arbitrary macro expansion, or user callbacks.


The opt-in live smoke example consumes process environment credentials through
validated API key and endpoint types. It saves correlation before creation and
identity before work under ignored `.plan/`, uses a five-minute provider TTL,
checks positive/negative command results and a file round trip, then confirms
scope cleanup with a separate observation. It requires `--run-live` and is never
executed by normal tests or CI. Package inclusion is limited to source/document
extensions; `.env` files are excluded even inside source directories.

## Awaitable tasks and input boundaries

Text enters through ScriptInput and becomes ShellScript before execution. Draft
command/scope input is private and becomes a validated request before the effect
boundary. client.run owns a temporary lifecycle; sandbox.run never creates or
deletes a resource. Both require a successful exit by default.

Macros return the same must-use builders and evaluate input expressions once in
source order. A saved creation reference remains identical through validation.
ScopeError<T, E> retains successful T if cleanup fails, without requiring T: Debug.
The report conversion preserves the same evidence. There is no async Drop cleanup.
