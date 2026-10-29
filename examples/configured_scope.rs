use daytona::{
    ApiKey, CheckedOutput, ConfigurationError, CpuAllocation, Create, CreateValidationError,
    Credentials, Daytona, DiskSize, ExecutionError, ExecutionTimeout, ImageSpec, MemorySize,
    NetworkPolicy, Resources, SandboxSource, SandboxTtl, ScopeError, ShellScript, ValidationError,
    WaitTimeout,
};
use std::io::Write;
use thiserror::Error;

#[derive(Debug, Error)]
enum ExampleError {
    #[error("pass --run-live to create a billable sandbox")]
    ConsentRequired,
    #[error(transparent)]
    Create(#[from] CreateValidationError),
    #[error(transparent)]
    Environment(#[from] std::env::VarError),
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
    #[error(transparent)]
    Scope(#[from] ScopeError<CheckedOutput, ExecutionError>),
    #[error(transparent)]
    Output(#[from] std::io::Error),
}

async fn run() -> Result<(), ExampleError> {
    if !std::env::args().any(|argument| argument == "--run-live") {
        return Err(ExampleError::ConsentRequired);
    }
    let key = ApiKey::new(std::env::var("DAYTONA_API_KEY")?)?;
    let client = Daytona::new(Credentials::ApiKey(key))?;
    let script = ShellScript::new("printf '%s\\n' 'Hello from Daytona'")?;
    let timeout = ExecutionTimeout::seconds(30)?;

    // The scope owns the sandbox. Your callback borrows it until work ends.
    let request = Create::draft()
        .ttl(SandboxTtl::minutes(5)?)
        .source(SandboxSource::Image(ImageSpec::new("FROM ubuntu:24.04")?))
        .network(NetworkPolicy::BlockAll)
        .resources(Resources::new(
            CpuAllocation::cores(1)?,
            MemorySize::gigabytes(1)?,
            DiskSize::gigabytes(3)?,
        ))
        .validate()?;
    let output = client
        .scope_from(request, async move |sandbox| {
            sandbox.run(script).execution_timeout(timeout).await
        })
        .readiness_timeout(WaitTimeout::seconds(120)?)
        .await?;

    // The output owns its text. The sandbox has already been deleted.
    write!(std::io::stdout().lock(), "{}", output.text())?;
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
