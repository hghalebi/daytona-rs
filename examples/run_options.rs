use daytona::{
    ApiKey, ConfigurationError, Credentials, Daytona, RunError, ScopeError, ValidationError,
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
    Scope(#[from] RunError),
    #[error(transparent)]
    Output(#[from] std::io::Error),
}

async fn run() -> Result<(), ExampleError> {
    if !std::env::args().any(|argument| argument == "--run-live") {
        return Err(ExampleError::ConsentRequired);
    }
    let key = ApiKey::new(std::env::var("DAYTONA_API_KEY")?)?;
    let client = Daytona::new(Credentials::ApiKey(key))?;
    let output = daytona::run!(&client, "printf '%s\\n' 'Hello from Daytona'", ttl: 5 min, execution_timeout: 30 sec).await?;

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
