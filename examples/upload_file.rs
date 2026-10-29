use daytona::*;
use std::io::Write;
use thiserror::Error;

#[derive(Debug, Error)]
enum WorkError {
    #[error(transparent)]
    Execution(#[from] ExecutionError),
    #[error(transparent)]
    Transfer(#[from] TransferError),
}
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
    Scope(#[from] ScopeError<TransferReceipt, WorkError>),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
#[tokio::main]
async fn main() -> Result<(), ExampleError> {
    if !std::env::args().any(|argument| argument == "--run-live") {
        return Err(ExampleError::ConsentRequired);
    }
    let key = ApiKey::new(std::env::var("DAYTONA_API_KEY")?)?;
    let client = Daytona::new(Credentials::ApiKey(key))?;
    let remote = SandboxPath::new("/tmp/daytona-lesson.txt")?;
    let local = LocalSourcePath::new("lesson-input.txt")?;
    // The setup command in the guide creates this small input file.
    let receipt = client
        .scope(async move |sandbox| Ok::<_, WorkError>(sandbox.upload(&local, &remote).await?))
        .ttl(SandboxTtl::minutes(5)?)
        .await?;
    writeln!(
        std::io::stdout().lock(),
        "Uploaded {} bytes; sandbox deleted",
        receipt.bytes().get()
    )?;
    Ok(())
}
