# Compiler contracts

A valid workflow borrows the client and moves its inputs exactly once:

```no_run
use daytona::*;
#[derive(Debug, thiserror::Error)]
enum ExampleError {
    #[error(transparent)] Validation(#[from] ValidationError),
    #[error(transparent)] Scope(#[from] ScopeError<ExecutionError>),
}
async fn run(client: &Daytona) -> Result<CheckedOutput, ExampleError> {
    let script = ShellScript::new("printf '%s\\n' hello")?;
    let execution_limit = ExecutionTimeout::seconds(30)?;
    Ok(client.sandboxes().ephemeral().ttl(SandboxTtl::minutes(10)?)
        .run(async move |sandbox| {
            sandbox.shell(script).execution_timeout(execution_limit).checked_output().await
        }).await?)
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
    let escaped = client.sandboxes().ephemeral().ttl(SandboxTtl::minutes(1)?)
        .run(async |sandbox| Ok::<_, ExecutionError>(sandbox)).await;
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
