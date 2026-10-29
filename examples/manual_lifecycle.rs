use daytona::*;
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
    CreateValidation(#[from] CreateValidationError),
    #[error(transparent)]
    Create(#[from] CreateError),
    #[error(transparent)]
    Scope(#[from] ScopeError<ExecutionError>),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[tokio::main]
async fn main() -> Result<(), ExampleError> {
    if !std::env::args().any(|argument| argument == "--run-live") {
        return Err(ExampleError::ConsentRequired);
    }
    let key = ApiKey::new(std::env::var("DAYTONA_API_KEY")?)?;
    let client = Daytona::new(Credentials::ApiKey(key))?;
    let request = client
        .sandboxes()
        .create()
        .ttl(SandboxTtl::minutes(5)?)
        .validate()?;
    let script = ShellScript::new("printf 'manual lifecycle complete\n'")?;
    let readiness = WaitTimeout::seconds(120)?;
    let cleanup_timeout = WaitTimeout::seconds(90)?;

    // Save intent before the first request. Cancellation cannot erase this file.
    std::fs::create_dir_all(".plan")?;
    let reference_path = format!(
        ".plan/manual-{}.json",
        request.reference().operation().as_str()
    );
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&reference_path)?;
    file.write_all(serde_json::to_string(request.reference())?.as_bytes())?;
    file.sync_all()?;
    writeln!(
        std::io::stdout().lock(),
        "Recovery reference: {reference_path}"
    )?;

    let receipt = request.submit(&client).await?;
    let ready = match receipt.wait(&client, readiness).await {
        Ok(ready) => ready,
        Err(failure) => {
            // A failed readiness wait still owns a resource that needs cleanup.
            let (receipt, cause) = match failure {
                WaitError::Deadline { receipt } => (receipt, SetupError::ReadinessDeadline),
                WaitError::Terminal { receipt } => (receipt, SetupError::ReadinessTerminal),
                WaitError::Observation { receipt, cause } => {
                    (receipt, SetupError::ReadinessObservation(cause))
                }
            };
            let cleanup = match client
                .sandbox(receipt.into_id())
                .delete(cleanup_timeout)
                .await
            {
                Ok(_) => None,
                Err(error) => Some(Box::new(error)),
            };
            return Err(ScopeError::Setup(ScopeSetupError { cause, cleanup }).into());
        }
    };
    let sandbox = ready.into_sandbox(&client);
    let work = sandbox.shell(script).checked_output().await;
    // Always attempt deletion after work, even when the command failed.
    let cleanup = sandbox.delete(cleanup_timeout).await;
    let report = match (work, cleanup) {
        (Ok(output), Ok(_)) => ScopeReport::Success(output),
        (Err(work), Ok(_)) => ScopeReport::WorkFailed(work),
        (Ok(output), Err(cleanup)) => ScopeReport::CleanupFailed { output, cleanup },
        (Err(work), Err(cleanup)) => ScopeReport::BothFailed { work, cleanup },
    };
    let output = report.into_result()?;
    write!(std::io::stdout().lock(), "{}", output.text())?;
    Ok(())
}
