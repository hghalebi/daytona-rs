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
    Scope(#[from] ScopeError<WorkError>),
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
    let local = LocalDestinationPath::new("lesson-output.txt")?;
    let script = ShellScript::new("printf 'hello\\n' > /tmp/daytona-lesson.txt")?;
    let receipt = client
        .sandboxes()
        .ephemeral()
        .ttl(SandboxTtl::minutes(5)?)
        .run(async move |sandbox| {
            // Prepare a known remote file so this lesson works by itself.
            sandbox.shell(script).checked_output().await?;
            Ok::<_, WorkError>(sandbox.download(&remote, &local).await?)
        })
        .await?;
    writeln!(
        std::io::stdout().lock(),
        "Downloaded {} bytes; sandbox deleted",
        receipt.bytes().get()
    )?;
    Ok(())
}
