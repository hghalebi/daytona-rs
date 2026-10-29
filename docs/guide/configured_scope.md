# Configure a sandbox and keep scoped cleanup

Use the first-command setup and dependencies. This live example creates a billable
sandbox from an Ubuntu image with explicit resources and blocked outbound network. Its command only
prints local text; it needs no model or package download.

```sh
cargo run --example configured_scope -- --run-live
```

Expected: `Hello from Daytona`, after deletion is confirmed. Build and validate
one request, then pass it and a callback to client.scope_from. The scope keeps its exact reference,
source, resources, and policies. Persist request.reference() before moving it
when restart recovery matters. Provider quotas and permissions are checked remotely.

This image example allows 120 seconds for readiness and 90 seconds for deletion
confirmation. A live check observed deletion still in progress after the default
30-second cleanup deadline; a later read confirmed absence. A deadline retains
the resource identity and never turns an unfinished deletion into success.

Custom resource sizes require an image source. A default or named snapshot keeps
its existing resources; validate() rejects combining it with custom allocations
and returns the original draft for correction. See Daytona’s
[resource guidance](https://www.daytona.io/docs/en/sandboxes/#resources).

Exercise: change the memory allocation. The same cleanup path still applies.
Do not use BlockAll for Pi or another task that needs external model access.
Next: [recover interrupted work](crate::guide::recovery).

## Complete example
