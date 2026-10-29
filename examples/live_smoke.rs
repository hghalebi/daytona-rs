//! Explicitly authorized live verification. Credentials are read from the process
//! environment; no dotenv files are read or embedded by the SDK or this example.
use daytona::*;
use serde::Serialize;
use std::{io::Write, path::Path};

#[derive(Debug, thiserror::Error)]
enum SmokeError {
    #[error("pass --run-live to authorize creating and deleting a cloud sandbox")]
    ConsentRequired,
    #[error("required environment variable is absent or invalid: {0}")]
    Environment(&'static str),
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("live scoped workflow failed; recovery reference is saved in .plan")]
    Scope(#[source] Box<ScopeError<WorkProof, ProbeError>>),
    #[error(transparent)]
    Operation(#[from] OperationError),
    #[error("independent readback did not confirm deletion")]
    StillPresent,
}
#[derive(Debug, thiserror::Error)]
enum ProbeError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Execution(#[from] ExecutionError),
    #[error(transparent)]
    Transfer(#[from] TransferError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("live execution returned unexpected output")]
    OutputMismatch,
    #[error("nonzero command status was not preserved")]
    ExitMismatch,
    #[error("uploaded/downloaded bytes did not match")]
    TransferMismatch,
}
#[derive(Serialize)]
enum Evidence {
    Confirmed,
}
#[derive(Serialize)]
struct WorkProof {
    execution: Evidence,
    nonzero_exit: Evidence,
    file_roundtrip: Evidence,
}
#[derive(Serialize)]
struct LiveReceipt<'a> {
    sandbox: &'a SandboxId,
    work: WorkProof,
    scope_cleanup: Evidence,
    independent_deletion_readback: Evidence,
}
fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), std::io::Error> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()
}
async fn probe(sandbox: &Sandbox<'_>, directory: &Path) -> Result<WorkProof, ProbeError> {
    write_json(&directory.join("sandbox.json"), sandbox.id())?;
    let output = sandbox
        .run(ShellScript::new("printf '%s\\n' 'daytona-rs-live-ok'")?)
        .execution_timeout(ExecutionTimeout::seconds(10)?)
        .await?;
    if output.text() != "daytona-rs-live-ok\n" {
        return Err(ProbeError::OutputMismatch);
    }
    match sandbox
        .run(ShellScript::new("exit 7")?)
        .execution_timeout(ExecutionTimeout::seconds(10)?)
        .await
    {
        Err(ExecutionError::NonZeroExit { output })
            if output.exit_code().is_some_and(|code| code.get() == 7) => {}
        Err(error) => return Err(ProbeError::Execution(error)),
        Ok(_) => return Err(ProbeError::ExitMismatch),
    }
    let source = directory.join("input.txt");
    let destination = directory.join("output.txt");
    std::fs::write(&source, "DAYTONA-SDK-FILE-PROBE\n")?;
    let remote = SandboxPath::new("/tmp/daytona-rs-smoke.txt")?;
    sandbox
        .upload(&LocalSourcePath::new(source)?, &remote)
        .await?;
    sandbox
        .download(&remote, &LocalDestinationPath::new(&destination)?)
        .await?;
    if std::fs::read_to_string(destination)? != "DAYTONA-SDK-FILE-PROBE\n" {
        return Err(ProbeError::TransferMismatch);
    }
    Ok(WorkProof {
        execution: Evidence::Confirmed,
        nonzero_exit: Evidence::Confirmed,
        file_roundtrip: Evidence::Confirmed,
    })
}
fn environment_or(name: &'static str, fallback: &str) -> Result<String, SmokeError> {
    match std::env::var(name) {
        Ok(value) => Ok(value),
        Err(std::env::VarError::NotPresent) => Ok(String::from(fallback)),
        Err(std::env::VarError::NotUnicode(_)) => Err(SmokeError::Environment(name)),
    }
}
#[tokio::main]
async fn main() -> Result<(), SmokeError> {
    if std::env::args().nth(1).as_deref() != Some("--run-live") {
        return Err(SmokeError::ConsentRequired);
    }
    let key =
        std::env::var("DAYTONA_API_KEY").map_err(|_| SmokeError::Environment("DAYTONA_API_KEY"))?;
    let api = Endpoint::new(&environment_or(
        "DAYTONA_API_URL",
        "https://app.daytona.io/api/",
    )?)?;
    let toolbox = Endpoint::new(&environment_or(
        "DAYTONA_TOOLBOX_URL",
        "https://proxy.app.daytona.io/toolbox/",
    )?)?;
    let client = Daytona::with_config(
        Credentials::ApiKey(ApiKey::new(key)?),
        ClientConfig::new(api, toolbox)?.request_timeout(RequestTimeout::seconds(30)?),
    )?;
    let directory =
        std::path::PathBuf::from(".plan").join(format!("live-smoke-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory)?;
    let scope = client
        .scope(async |sandbox| probe(sandbox, &directory).await)
        .ttl(SandboxTtl::minutes(5)?)
        .readiness_timeout(WaitTimeout::seconds(120)?)
        .cleanup_timeout(WaitTimeout::seconds(90)?);
    write_json(
        &directory.join("creation-reference.json"),
        scope.reference(),
    )?;
    writeln!(
        std::io::stdout().lock(),
        "Recovery directory: {}",
        directory.display()
    )?;
    let work = scope
        .await
        .map_err(|error| SmokeError::Scope(Box::new(error)))?;
    let id: SandboxId =
        serde_json::from_reader(std::fs::File::open(directory.join("sandbox.json"))?)?;
    let sandbox = client.sandbox(id);
    match sandbox.observe().await? {
        None => {}
        Some(observation) if matches!(observation.state(), RemoteState::Destroyed) => {}
        Some(_) => return Err(SmokeError::StillPresent),
    }
    write_json(
        &directory.join("receipt.json"),
        &LiveReceipt {
            sandbox: sandbox.id(),
            work,
            scope_cleanup: Evidence::Confirmed,
            independent_deletion_readback: Evidence::Confirmed,
        },
    )?;
    writeln!(
        std::io::stdout().lock(),
        "PASS: live execution, nonzero exit, file transfer, scope cleanup, independent deletion readback"
    )?;
    Ok(())
}
