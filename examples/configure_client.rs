use daytona::{
    ApiKey, ClientConfig, ConfigurationError, Credentials, Daytona, RequestTimeout, ValidationError,
};
use std::io::Write;
use thiserror::Error;

#[derive(Debug, Error)]
enum ExampleError {
    #[error(transparent)]
    Environment(#[from] std::env::VarError),
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
    #[error(transparent)]
    Output(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), ExampleError> {
    let key = ApiKey::new(std::env::var("DAYTONA_API_KEY")?)?;
    let config = ClientConfig::cloud()?.request_timeout(RequestTimeout::seconds(30)?);
    let client = Daytona::with_config(Credentials::ApiKey(key), config)?;
    // Construction is local. It has not tested the key against Daytona.
    drop(client);
    writeln!(
        std::io::stdout().lock(),
        "Client configured; no API request sent"
    )?;
    Ok(())
}
