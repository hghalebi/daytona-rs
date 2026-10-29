//! Run locally to exercise every report branch; pass --run-live for real scoped work.
use daytona::*;
use std::io::Write;
use thiserror::Error;

#[derive(Debug, Error)]
enum WorkError {
    #[error(transparent)]
    Execution(#[from] ExecutionError),
    #[error("the callback deadline elapsed; remote work may still run")]
    Deadline,
}
#[derive(Debug, Error)]
enum ExampleError {
    #[error(transparent)]
    LiveScope(#[from] ScopeError<CheckedOutput, WorkError>),
    #[error(transparent)]
    Scope(#[from] ScopeError<(), WorkError>),
    #[error(transparent)]
    Output(#[from] std::io::Error),
    #[error("saving output and sandbox cleanup both failed")]
    OutputAndCleanup {
        #[source]
        output: std::io::Error,
        cleanup: CleanupError,
    },
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
    #[error(transparent)]
    Environment(#[from] std::env::VarError),
    #[error(transparent)]
    Setup(#[from] ScopeSetupError),
    #[error("local report simulation produced the wrong outcome")]
    Contract,
}

fn handle<T>(
    report: ScopeReport<T, WorkError>,
    destination: &mut impl Write,
    text: impl Fn(&T) -> &str,
) -> Result<(), ExampleError> {
    match report {
        ScopeReport::Success(output) => {
            destination.write_all(text(&output).as_bytes())?;
            Ok(())
        }
        ScopeReport::CleanupFailed { output, cleanup } => {
            match destination.write_all(text(&output).as_bytes()) {
                Ok(()) => Err(ScopeError::Cleanup {
                    output: (),
                    cleanup,
                }
                .into()),
                Err(output) => Err(ExampleError::OutputAndCleanup { output, cleanup }),
            }
        }
        ScopeReport::WorkFailed(work) => Err(ExampleError::Scope(ScopeError::Work(work))),
        ScopeReport::BothFailed { work, cleanup } => {
            Err(ExampleError::Scope(ScopeError::Both { work, cleanup }))
        }
    }
}
fn simulated_cleanup() -> Result<CleanupError, ValidationError> {
    Ok(CleanupError::Deadline {
        sandbox: SandboxId::new("local-simulation")?,
        acceptance: DeleteAcceptance::Acknowledged,
    })
}
fn simulate() -> Result<(), ExampleError> {
    let reports = [
        ScopeReport::Success(String::from("saved work")),
        ScopeReport::WorkFailed(WorkError::Deadline),
        ScopeReport::CleanupFailed {
            output: String::from("saved work"),
            cleanup: simulated_cleanup()?,
        },
        ScopeReport::BothFailed {
            work: WorkError::Deadline,
            cleanup: simulated_cleanup()?,
        },
    ];
    for (index, report) in reports.into_iter().enumerate() {
        let mut saved = Vec::new();
        let result = handle(report, &mut saved, String::as_str);
        let correct = match (index, result) {
            (0, Ok(())) | (2, Err(ExampleError::Scope(ScopeError::Cleanup { .. }))) => {
                saved == b"saved work"
            }
            (1, Err(ExampleError::Scope(ScopeError::Work(_))))
            | (3, Err(ExampleError::Scope(ScopeError::Both { .. }))) => saved.is_empty(),
            _ => false,
        };
        if !correct {
            return Err(ExampleError::Contract);
        }
    }
    writeln!(
        std::io::stdout().lock(),
        "Four report outcomes handled; useful output retained. No cloud requests."
    )?;
    Ok(())
}
#[tokio::main]
async fn main() -> Result<(), ExampleError> {
    if !std::env::args().any(|arg| arg == "--run-live") {
        return simulate();
    }
    let client = Daytona::new(Credentials::ApiKey(ApiKey::new(std::env::var(
        "DAYTONA_API_KEY",
    )?)?))?;
    let script = ShellScript::new("printf 'saved work\\n'")?;
    let work_limit = ExecutionTimeout::seconds(30)?;
    let report = client
        .scope(async move |sandbox| {
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(40),
                sandbox.run(script).execution_timeout(work_limit),
            )
            .await
            .map_err(|_| WorkError::Deadline)??;
            Ok::<CheckedOutput, WorkError>(result)
        })
        .ttl(SandboxTtl::minutes(5)?)
        .report()
        .await?;
    handle(report, &mut std::io::stdout().lock(), CheckedOutput::text)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct FailedWriter;
    impl Write for FailedWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("synthetic write failure"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn persistence_and_cleanup_failures_survive() -> Result<(), ExampleError> {
        simulate()?;
        let report = ScopeReport::CleanupFailed {
            output: String::from("useful"),
            cleanup: simulated_cleanup()?,
        };
        match handle(report, &mut FailedWriter, String::as_str) {
            Err(ExampleError::OutputAndCleanup { cleanup, .. })
                if cleanup.sandbox_id().as_str() == "local-simulation" =>
            {
                Ok(())
            }
            _ => Err(ExampleError::Contract),
        }
    }
}
