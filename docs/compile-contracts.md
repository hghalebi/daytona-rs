# Compiler contracts

## Awaitable tasks and macro contracts

Valid examples of each syntax appear in the compiled macro guide. These negative
cases protect option syntax, domain types, ownership and receiver authority.

A task cannot be consumed twice.

```compile_fail,E0382
use daytona::*;
async fn invalid(client: &Daytona) { let task = client.run("true"); let first = task.await; let second = task.await; drop((first, second)); }
```

Timeouts and lifetimes remain distinct.

```compile_fail,E0308
use daytona::*;
fn invalid(client: &Daytona, limit: ExecutionTimeout) { let task = daytona::run!(client, "true", ttl: limit); drop(task); }
```

Duplicate options are rejected.

```compile_fail
use daytona::*;
fn invalid(client: &Daytona) { let task = daytona::run!(client, "true", ttl: 1 min, ttl: 2 min); drop(task); }
```

Execution time uses seconds.

```compile_fail
use daytona::*;
fn invalid(client: &Daytona) { let task = daytona::run!(client, "true", execution_timeout: 2 min); drop(task); }
```

Unknown options are rejected.

```compile_fail
use daytona::*;
fn invalid(client: &Daytona) { let task = daytona::run!(client, "true", retry: true); drop(task); }
```

A scope requires work.

```compile_fail
use daytona::*;
fn invalid(client: &Daytona) { let task = daytona::scope!(client, ttl: 5 min); drop(task); }
```

A scope lifetime appears once.

```compile_fail
use daytona::*;
fn invalid(client: &Daytona) { let task = daytona::scope!(client, work: async |_| Ok::<(), ExecutionError>(()), ttl: 1 min, ttl: 2 min); drop(task); }
```

An existing sandbox command has no lifetime setter.

```compile_fail,E0599
use daytona::*;
fn invalid(sandbox: &Sandbox<'_>) { let command = daytona::run!(sandbox, "true", ttl: 5 min); drop(command); }
```

Literal limits must fit their integer representation.

```compile_fail
use daytona::*;
fn invalid() { let value = daytona::ttl!(4294967296 min); drop(value); }
```

Fractional limits are not silently rounded.

```compile_fail,E0308
use daytona::*;
fn invalid() { let value = daytona::execution_timeout!(1.5 sec); drop(value); }
```

Negative lifetimes are rejected.

```compile_fail
use daytona::*;
fn invalid() { let value = daytona::ttl!(-1 min); drop(value); }
```

Zero is validated at runtime without remote effects:

```rust
# fn main() -> Result<(), daytona::ValidationError> {
match daytona::ttl!(0 min) {
    Err(daytona::ValidationError::Zero { .. }) => {}
    _ => return Err(daytona::ValidationError::Range { field: "zero TTL contract" }),
}
# Ok(())
# }
```


A valid workflow borrows the client and moves its inputs exactly once:

```no_run
use daytona::*;
#[derive(Debug, thiserror::Error)]
enum ExampleError {
    #[error(transparent)] Validation(#[from] ValidationError),
    #[error(transparent)] Scope(#[from] ScopeError<CheckedOutput, ExecutionError>),
}
async fn run(client: &Daytona) -> Result<CheckedOutput, ExampleError> {
    let script = ShellScript::new("printf '%s\\n' hello")?;
    let execution_limit = ExecutionTimeout::seconds(30)?;
    Ok(client.scope(async move |sandbox| {
            sandbox.run(script).execution_timeout(execution_limit).await
        }).ttl(SandboxTtl::minutes(10)?)
        .await?)
}
```

Identities are nominal:

```compile_fail,E0308
use daytona::{Daytona, SessionId};
fn wrong(client: &Daytona, session: SessionId) { client.sandbox(session); }
```

Representations are private:

```compile_fail,E0423
use daytona::SandboxId;
let forged = SandboxId(String::new());
```

Only validated requests submit:

```compile_fail,E0599
use daytona::{Create, Daytona};
async fn wrong(client: &Daytona) { Create::draft().submit(client).await; }
```

Validation consumes the draft:

```compile_fail,E0382
use daytona::Create;
let draft = Create::draft();
let first = draft.validate();
let second = draft.validate();
```

An uncertain outcome cannot be replayed:

```compile_fail,E0599
use daytona::{UncertainCreation, Daytona};
async fn wrong(client: &Daytona, recovery: UncertainCreation) { recovery.submit(client).await; }
```

Receipts cannot be deserialized as evidence:

```compile_fail,E0277
use daytona::CreationReceipt;
let forged: CreationReceipt = serde_json::from_str("{\"id\":\"abc\"}")?;
# Ok::<(), serde_json::Error>(())
```

A scope cannot lend out its temporary handle:

```compile_fail
use daytona::*;
async fn wrong(client: &Daytona) -> Result<(), ValidationError> {
    let escaped = client.scope(async |sandbox| Ok::<_, ExecutionError>(sandbox)).ttl(SandboxTtl::minutes(1)?)
        .await;
    Ok(())
}
```

A positive consuming creation path:

```no_run
use daytona::*;
async fn create(client: &Daytona, ttl: SandboxTtl) -> Result<Sandbox<'_>, CreateError> {
    let request = match Create::draft().ttl(ttl).validate() {
        Ok(request) => request,
        Err(failure) => { let (draft, violation) = failure.into_parts(); return correct(draft, violation).await; }
    };
    let receipt = request.submit(client).await?;
    Ok(client.sandbox(receipt.into_id()))
}
# async fn correct<'a>(_: Create<Draft>, _: ValidationError) -> Result<Sandbox<'a>, CreateError> { loop { std::future::pending::<()>().await; } }
```

## A borrowed scope cannot give away its sandbox

The callback can inspect the identity. It cannot consume the scope's handle.

```compile_fail,E0507
use daytona::{Daytona, SandboxTtl, SandboxId, ValidationError};
async fn invalid(client: &Daytona) -> Result<(), ValidationError> {
    let result = client.scope(async |sandbox| Ok::<SandboxId, ValidationError>(sandbox.into_id())).ttl(SandboxTtl::minutes(5)?)
        .await;
    drop(result);
    Ok(())
}
```

An independently owned handle can return its identity without a request.

```no_run
use daytona::{Daytona, SandboxId};
fn keep_identity(client: &Daytona, id: SandboxId) -> SandboxId {
    client.sandbox(id).into_id()
}
```

## Configured scopes require validated, owned intent

```compile_fail,E0308
use daytona::{Create, Daytona};
fn invalid(client: &Daytona) {
    let scope = client.scope_from(Create::draft(), async |_| Ok::<(), daytona::ExecutionError>(()));
    drop(scope);
}
```

```compile_fail,E0382
use daytona::{Create, Daytona, Validated};
fn invalid(client: &Daytona, request: Create<Validated>) {
    let scope = client.scope_from(request, async |_| Ok::<(), daytona::ExecutionError>(()));
    drop(request);
    drop(scope);
}
```

```no_run
use daytona::{Create, Daytona, Validated};
fn valid(client: &Daytona, request: Create<Validated>) {
    let scope = client.scope_from(request, async |_| Ok::<(), daytona::ExecutionError>(()));
    let reference = scope.reference();
    let _operation = reference.operation();
}
```
