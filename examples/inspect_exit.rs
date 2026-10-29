use daytona::{
    ApiKey, ConfigurationError, Credentials, Daytona, ExecutionError, ExecutionTimeout, SandboxTtl,
    ScopeError, ShellScript, ValidationError,
};
use std::io::Write;
use thiserror::Error;

#[derive(Debug, Error)]
enum ExampleError {
    #[error("pass --run-live to create a billable sandbox")]
    ConsentRequired,
    #[error(transparent)]
    Environment(#[from] std::env::VarError),
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
    #[error(transparent)]
    Scope(#[from] ScopeError<ExecutionError>),
    #[error("expected exit code 7 from the demonstration command")]
    UnexpectedExit,
    #[error(transparent)]
    Output(#[from] std::io::Error),
}

async fn run() -> Result<(), ExampleError> {
    if !std::env::args().any(|argument| argument == "--run-live") {
        return Err(ExampleError::ConsentRequired);
    }
    let key = ApiKey::new(std::env::var("DAYTONA_API_KEY")?)?;
    let client = Daytona::new(Credentials::ApiKey(key))?;
    let script = ShellScript::new("exit 7")?;
    let timeout = ExecutionTimeout::seconds(30)?;

    // The scope owns the sandbox. Your callback borrows it until work ends.
    let output = client
        .sandboxes()
        .ephemeral()
        .ttl(SandboxTtl::minutes(5)?)
        .run(async move |sandbox| {
            sandbox
                .shell(script)
                .execution_timeout(timeout)
                .output()
                .await
        })
        .await?;

    if !output.exit_code().is_some_and(|code| code.get() == 7) {
        return Err(ExampleError::UnexpectedExit);
    }
    // Cleanup is complete before we inspect the owned result.
    writeln!(
        std::io::stdout().lock(),
        "Handled exit code 7; sandbox deleted"
    )?;
    drop(output);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<std::process::ExitCode, std::io::Error> {
    match run().await {
        Ok(()) => Ok(std::process::ExitCode::SUCCESS),
        Err(error) => {
            writeln!(std::io::stderr().lock(), "{error}")?;
            if let ExampleError::Scope(scope) = &error {
                if let ScopeError::Setup(setup) = scope {
                    writeln!(std::io::stderr().lock(), "Setup: {}", setup.cause)?;
                }
                if let Some(work) = scope.work_error() {
                    writeln!(std::io::stderr().lock(), "Work: {work}")?;
                }
                if let Some(cleanup) = scope.cleanup_error() {
                    writeln!(
                        std::io::stderr().lock(),
                        "Cleanup: {cleanup}; sandbox {}",
                        cleanup.sandbox_id().as_str()
                    )?;
                }
            }
            Ok(std::process::ExitCode::FAILURE)
        }
    }
}
