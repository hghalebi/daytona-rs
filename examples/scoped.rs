use daytona::{
    CheckedOutput, Daytona, ExecutionError, ExecutionTimeout, SandboxTtl, ScopeError, ShellScript,
    ValidationError,
};
use thiserror::Error;
#[derive(Debug, Error)]
pub enum ExampleError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Scope(#[from] ScopeError<ExecutionError>),
}
pub async fn run_example(client: &Daytona) -> Result<CheckedOutput, ExampleError> {
    let script = ShellScript::new("printf '%s\\n' 'Hello from Daytona'")?;
    let lifetime = SandboxTtl::minutes(10)?;
    let execution_limit = ExecutionTimeout::seconds(30)?;
    Ok(client
        .sandboxes()
        .ephemeral()
        .ttl(lifetime)
        .run(async move |sandbox| {
            sandbox
                .shell(script)
                .execution_timeout(execution_limit)
                .checked_output()
                .await
        })
        .await?)
}
fn main() {}
