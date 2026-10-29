use sdk::{
    ApiKey, CheckedOutput, ConfigurationError, Credentials, Daytona, ExecutionError, RunError,
    SandboxTtl, ScopeError, ValidationError,
};
use std::io::Write;

#[derive(Debug, thiserror::Error)]
enum ConsumerError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
async fn borrowed<'a>(
    client: &'a Daytona,
    text: &'a str,
) -> Result<&'a str, ScopeError<&'a str, ExecutionError>> {
    sdk::scope!(client, work: async |_| Ok::<_, ExecutionError>(text), ttl: 5 min).await
}
async fn command(client: &Daytona) -> Result<CheckedOutput, RunError> {
    sdk::run!(client, "true", execution_timeout: 30 sec, ttl: 5 min).await
}
fn main() -> Result<(), ConsumerError> {
    let client = Daytona::new(Credentials::ApiKey(ApiKey::new("local-consumer-fixture")?))?;
    let task = sdk::run!(&client, String::from("true"), ttl: SandboxTtl::minutes(5)?, execution_timeout: 30 sec);
    let reference = task.reference();
    let operation = reference.operation();
    let _identity = operation.as_str();
    drop(task);
    let text = String::from("borrowed application text");
    drop(borrowed(&client, &text));
    drop(command(&client));
    let ttl = sdk::ttl!(5 min)?;
    let limit = sdk::execution_timeout!(30 sec)?;
    drop(client.run("true").ttl(ttl).execution_timeout(limit));
    writeln!(
        std::io::stdout().lock(),
        "Consumer verified; no cloud request sent"
    )?;
    Ok(())
}
