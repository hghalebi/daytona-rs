# Short calls and readable options

Start with client.run(script).await for one temporary-sandbox command.
Use client.scope(callback).await when work needs several commands or file transfers.
Inside a scope, sandbox.run(script).await uses the borrowed sandbox and does not delete it.

## Named options

The run! macro returns the same task builder as the method API:

```no_run
use daytona::{CheckedOutput, Daytona, RunError};

async fn run(client: &Daytona) -> Result<CheckedOutput, RunError> {
    daytona::run!(
        client,
        "python --version",
        execution_timeout: 30 sec,
        ttl: 5 min,
    ).await
}
```

Options may appear in either order, once each. A trailing comma is allowed.
Use min for TTL and sec for execution time. Other units are rejected.
Integer literals must fit u32. Zero returns a validation error when awaited.
The macro does not claim compile-time value validation.

Typed settings work in the same positions. Dynamic quantities use the normal
constructors:

```no_run
use daytona::*;
#[derive(Debug, thiserror::Error)]
enum ExampleError {
    #[error(transparent)] Validation(#[from] ValidationError),
    #[error(transparent)] Run(#[from] RunError),
}
async fn configured(client: &Daytona, minutes: u32) -> Result<CheckedOutput, ExampleError> {
    let lifetime = SandboxTtl::minutes(minutes)?;
    Ok(daytona::run!(client, "true", ttl: lifetime, execution_timeout: 30 sec).await?)
}
```

The standalone ttl! and execution_timeout! macros return Results immediately:

```
use daytona::{SandboxTtl, ExecutionTimeout, ValidationError};
fn settings() -> Result<(SandboxTtl, ExecutionTimeout), ValidationError> {
    Ok((daytona::ttl!(5 min)?, daytona::execution_timeout!(30 sec)?))
}
# settings()?;
# Ok::<(), ValidationError>(())
```

## Several operations

```no_run
use daytona::*;
async fn task(client: &Daytona) -> Result<CheckedOutput, RunError> {
    daytona::scope!(
        client,
        ttl: 5 min,
        work: async |sandbox| {
            sandbox.run("mkdir -p /tmp/work").await?;
            sandbox.run("ls /tmp/work").await
        },
    ).await
}
```

Only work and ttl are accepted by scope!. TTL may appear before or after work.
The callback body is ordinary Rust. Its return value must not borrow the temporary
sandbox. Local captures may borrow application data. No Send requirement is added.

Use the method builder for additional settings such as readiness, cleanup or
independent command budgets. There is no macro-only capability:

```no_run
use daytona::*;
async fn task(client: &Daytona, ttl: SandboxTtl) -> Result<CheckedOutput, RunError> {
    client.scope(async |sandbox| sandbox.run("true").await).ttl(ttl).await
}
```

run! also works on an existing sandbox. In that case ttl is not available and
the SDK sends only the command request. It does not create or delete a sandbox.

## Evaluation, errors and recovery

Receiver, script and options evaluate once, in source order, when constructing
the task. SDK network work starts only when its future is polled. User expressions
still have their own normal effects. A question mark that you write inside an
argument keeps its ordinary Rust meaning.

SDK-generated unit validation errors are stored in the builder until await.
Typed setters can replace a failed setting before execution. Invalid scripts and
settings never trigger sandbox creation. Preparation errors retain owned drafts
and their references for explicit correction.

Keep the builder when recovery matters:

```no_run
use daytona::*;
#[derive(Debug, thiserror::Error)]
enum ExampleError {
    #[error(transparent)] Json(#[from] serde_json::Error),
    #[error(transparent)] Io(#[from] std::io::Error),
    #[error(transparent)] Run(#[from] RunError),
}
async fn saved(client: &Daytona) -> Result<CheckedOutput, ExampleError> {
    let task = daytona::run!(client, "true", ttl: 5 min);
    let mut file = std::fs::File::create_new("creation-reference.json")?;
    serde_json::to_writer(&mut file, task.reference())?;
    file.sync_all()?;
    Ok(task.await?)
}
```

This small example saves correlation data. For a complete application record,
also retain the intended environment and configuration as shown in
[restart recovery](crate::guide::recovery). No implicit file is written by the SDK.

Awaiting a task allocates one boxed future. The execute() method exposes the
underlying async operation. For a combinator that requires Future rather than
IntoFuture, use execute() or import std::future::IntoFuture and call into_future().

Scripts are preserved exactly. There is no shell interpolation, quoting,
credential lookup, automatic retry or hidden runtime. Successful output remains
available if deletion fails. Dropping a polled task does not perform asynchronous
cleanup; a TTL is not deletion evidence.
