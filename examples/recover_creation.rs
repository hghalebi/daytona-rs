use daytona::{
    ApiKey, ConfigurationError, CreationReference, Credentials, Daytona, OperationError,
    Reconciliation, ValidationError,
};
use std::{io::Write, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
enum ExampleError {
    #[error("provide the path of a saved creation reference")]
    MissingReference,
    #[error(transparent)]
    Environment(#[from] std::env::VarError),
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
    #[error(transparent)]
    Lookup(#[from] OperationError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[tokio::main]
async fn main() -> Result<(), ExampleError> {
    let path = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or(ExampleError::MissingReference)?,
    );
    let reference: CreationReference = serde_json::from_slice(&std::fs::read(path)?)?;
    let key = ApiKey::new(std::env::var("DAYTONA_API_KEY")?)?;
    let client = Daytona::new(Credentials::ApiKey(key))?;
    // Reconciliation only reads. It never repeats the original create request.
    match reference.reconcile(&client).await? {
        Reconciliation::Found(receipt) => {
            writeln!(
                std::io::stdout().lock(),
                "Found sandbox: {}",
                receipt.sandbox_id().as_str()
            )?;
        }
        Reconciliation::Unresolved => {
            writeln!(
                std::io::stdout().lock(),
                "Unresolved; do not replay creation"
            )?;
        }
    }
    Ok(())
}
