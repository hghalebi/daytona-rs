//! Two bounded jobs borrow one supervisor-owned client and retain both outcomes.
use daytona::*;
use std::{io::Write, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
enum JobError {
    #[error(transparent)]
    Execution(#[from] ExecutionError),
    #[error("work deadline elapsed; remote work may still be running")]
    Deadline,
}
#[derive(Debug, Error)]
enum ExampleError {
    #[error("pass --run-live to create two billable sandboxes")]
    Consent,
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
    #[error(transparent)]
    Environment(#[from] std::env::VarError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Setup(#[from] ScopeSetupError),
    #[error(transparent)]
    Scope(#[from] ScopeError<JobError>),
    #[error("saving output and cleanup both failed")]
    SaveAndCleanup {
        #[source]
        save: std::io::Error,
        cleanup: CleanupError,
    },
    #[error("both jobs failed; both errors retained")]
    Both {
        #[source]
        first: Box<ExampleError>,
        second: Box<ExampleError>,
    },
}
struct Job {
    script: ShellScript,
    budget: CommandBudget,
}
struct Supervisor {
    client: Daytona,
}
struct PreparedJob<'client> {
    scope: Ephemeral<'client, HasTtl>,
    job: Job,
    artifact: PathBuf,
}
impl Supervisor {
    // Preparation finishes before either cancellable job future starts.
    fn prepare(&self, job: Job) -> Result<PreparedJob<'_>, ExampleError> {
        let scope = self
            .client
            .sandboxes()
            .ephemeral()
            .ttl(SandboxTtl::minutes(5)?);
        let directory =
            PathBuf::from(".plan").join(format!("job-{}", scope.reference().operation().as_str()));
        std::fs::create_dir_all(&directory)?;
        let mut file = std::fs::File::create_new(directory.join("creation-reference.json"))?;
        serde_json::to_writer(&mut file, scope.reference())?;
        file.sync_all()?;
        Ok(PreparedJob {
            scope,
            job,
            artifact: directory.join("result.txt"),
        })
    }
}
impl PreparedJob<'_> {
    async fn run(self) -> Result<(), ExampleError> {
        let job = self.job;
        let report = self
            .scope
            .run_report(async move |sandbox| {
                tokio::time::timeout(
                    std::time::Duration::from_secs(45),
                    sandbox
                        .shell(job.script)
                        .budget(job.budget)
                        .checked_output(),
                )
                .await
                .map_err(|_| JobError::Deadline)?
                .map_err(JobError::from)
            })
            .await?;
        // Output owns its data. Save it even if deletion was not confirmed.
        match report {
            ScopeReport::Success(output) => {
                std::fs::write(self.artifact, output.text())?;
                Ok(())
            }
            ScopeReport::CleanupFailed { output, cleanup } => {
                match std::fs::write(self.artifact, output.text()) {
                    Ok(()) => Err(ScopeError::Cleanup(cleanup).into()),
                    Err(save) => Err(ExampleError::SaveAndCleanup { save, cleanup }),
                }
            }
            ScopeReport::WorkFailed(work) => Err(ScopeError::Work(work).into()),
            ScopeReport::BothFailed { work, cleanup } => {
                Err(ScopeError::Both { work, cleanup }.into())
            }
        }
    }
}
#[tokio::main]
async fn main() -> Result<(), ExampleError> {
    if !std::env::args().any(|arg| arg == "--run-live") {
        return Err(ExampleError::Consent);
    }
    let config = ClientConfig::cloud()?
        .concurrency(ConcurrencyLimit::new(2)?)
        .lifecycle_capacity(LifecycleCapacity::Wait {
            pending: PendingLifecycleLimit::new(2)?,
            timeout: RequestTimeout::seconds(10)?,
        });
    let supervisor = Supervisor {
        client: Daytona::with_config(
            Credentials::ApiKey(ApiKey::new(std::env::var("DAYTONA_API_KEY")?)?),
            config,
        )?,
    };
    let first = supervisor.prepare(Job {
        script: ShellScript::new("printf 'first artifact\\n'")?,
        budget: CommandBudget::new(ExecutionTimeout::seconds(30)?, RequestTimeout::seconds(40)?),
    })?;
    let second = supervisor.prepare(Job {
        script: ShellScript::new("printf 'second artifact\\n'")?,
        budget: CommandBudget::new(ExecutionTimeout::seconds(30)?, RequestTimeout::seconds(40)?),
    })?;
    // Fixed batch of two: no unbounded queue, cloned client, or detached tasks.
    match tokio::join!(first.run(), second.run()) {
        (Ok(()), Ok(())) => writeln!(
            std::io::stdout().lock(),
            "Both artifacts saved; both sandboxes deleted"
        )?,
        (Err(first), Err(second)) => {
            return Err(ExampleError::Both {
                first: Box::new(first),
                second: Box::new(second),
            });
        }
        (Err(error), Ok(())) | (Ok(()), Err(error)) => return Err(error),
    }
    Ok(())
}
