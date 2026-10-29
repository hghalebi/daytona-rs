use daytona::{CheckedOutput, CleanupError, ExecutionError, ScopeError, ScopeReport};
use std::io::Write;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExampleError {
    #[error(transparent)]
    Scope(#[from] ScopeError<ExecutionError>),
    #[error(transparent)]
    Output(#[from] std::io::Error),
    #[error("output could not be written and sandbox cleanup also failed")]
    OutputAndCleanup {
        #[source]
        output: std::io::Error,
        cleanup: CleanupError,
    },
}

// Call this with the report returned by scope.run_report(...).await?.
// It preserves a cleanup failure even if writing the useful output also fails.
pub fn handle(report: ScopeReport<CheckedOutput, ExecutionError>) -> Result<(), ExampleError> {
    match report {
        ScopeReport::Success(output) => {
            write!(std::io::stdout().lock(), "{}", output.text())?;
            Ok(())
        }
        ScopeReport::CleanupFailed { output, cleanup } => {
            match write!(std::io::stdout().lock(), "{}", output.text()) {
                Ok(()) => Err(ScopeError::Cleanup(cleanup).into()),
                Err(output) => Err(ExampleError::OutputAndCleanup { output, cleanup }),
            }
        }
        ScopeReport::WorkFailed(work) => Err(ScopeError::Work(work).into()),
        ScopeReport::BothFailed { work, cleanup } => Err(ScopeError::Both { work, cleanup }.into()),
    }
}
fn main() -> Result<(), std::io::Error> {
    writeln!(
        std::io::stdout().lock(),
        "Handler compiled; no remote failure was simulated"
    )
}
