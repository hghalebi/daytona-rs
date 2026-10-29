use daytona::{SandboxTtl, ShellScript, ValidationError};
use std::io::Write;
use thiserror::Error;

#[derive(Debug, Error)]
enum ExampleError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Output(#[from] std::io::Error),
    #[error("an empty script unexpectedly passed validation")]
    EmptyScriptAccepted,
}

fn main() -> Result<(), ExampleError> {
    let script = ShellScript::new("printf 'hello'")?;
    let ttl = SandboxTtl::minutes(5)?;
    writeln!(std::io::stdout().lock(), "Script: {}", script.as_str())?;
    writeln!(
        std::io::stdout().lock(),
        "Lifetime: {} minutes",
        ttl.as_minutes()
    )?;
    // Invalid input is a value you can handle, not a panic.
    match ShellScript::new("") {
        Err(ValidationError::InvalidText { .. }) => {
            writeln!(std::io::stdout().lock(), "Empty script rejected")?;
        }
        Err(error) => return Err(error.into()),
        Ok(_) => return Err(ExampleError::EmptyScriptAccepted),
    }
    Ok(())
}
