use daytona::{CheckedOutput, Daytona, RunError};

/// Runs one command and returns owned output after confirmed sandbox deletion.
pub async fn run_example(client: &Daytona) -> Result<CheckedOutput, RunError> {
    client.run("printf '%s\\n' 'Hello from Daytona'").await
}
fn main() {}
