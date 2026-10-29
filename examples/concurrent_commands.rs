use daytona::{
    ApiKey, ConfigurationError, Credentials, Daytona, ExecutionError, ExecutionTimeout, SandboxTtl,
    ScopeError, ShellScript, ValidationError,
};
use std::io::Write;
use thiserror::Error;

#[derive(Debug, Error)]
enum WorkError {
    #[error(transparent)]
    One(#[from] ExecutionError),
    #[error("both commands failed")]
    Both {
        #[source]
        first: Box<ExecutionError>,
        second: Box<ExecutionError>,
    },
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
    Output(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), ExampleError> {
    if !std::env::args().any(|argument| argument == "--run-live") {
        return Err(ExampleError::ConsentRequired);
    }
    let key = ApiKey::new(std::env::var("DAYTONA_API_KEY")?)?;
    let client = Daytona::new(Credentials::ApiKey(key))?;
    let first_script = ShellScript::new("printf 'first\\n'")?;
    let second_script = ShellScript::new("printf 'second\\n'")?;
    let first_timeout = ExecutionTimeout::seconds(30)?;
    let second_timeout = ExecutionTimeout::seconds(30)?;
    let (first, second) = client
        .sandboxes()
        .ephemeral()
        .ttl(SandboxTtl::minutes(5)?)
        .run(async move |sandbox| {
            let first = sandbox
                .shell(first_script)
                .execution_timeout(first_timeout)
                .checked_output();
            let second = sandbox
                .shell(second_script)
                .execution_timeout(second_timeout)
                .checked_output();
            match tokio::join!(first, second) {
                (Ok(first), Ok(second)) => Ok((first, second)),
                (Err(first), Err(second)) => Err(WorkError::Both {
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (Err(error), Ok(_)) | (Ok(_), Err(error)) => Err(WorkError::One(error)),
            }
        })
        .await?;
    write!(
        std::io::stdout().lock(),
        "{}{}",
        first.text(),
        second.text()
    )?;
    Ok(())
}
