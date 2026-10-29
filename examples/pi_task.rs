//! Run Pi once, check its generated program, save the file, and delete the sandbox.
use daytona::*;
use std::{io::Write, path::Path};
use thiserror::Error;

#[derive(Debug, Error)]
enum ExampleError {
    #[error("pass --run-live to create a billable sandbox and use your model account")]
    ConsentRequired,
    #[error(transparent)]
    Environment(#[from] std::env::VarError),
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("PI_AUTH_FILE must contain only one openai-codex credential object")]
    AuthFile,
    #[error(transparent)]
    Scope(#[from] ScopeError<TaskError>),
    #[error(transparent)]
    Operation(#[from] OperationError),
    #[error("the independent readback did not confirm sandbox deletion")]
    StillPresent,
}

#[derive(Debug, Error)]
enum TaskError {
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
    #[error("the generated program did not print 30")]
    OutputMismatch,
}

// This validates the exported file's shape. Pi and its provider check the login.
struct PiAuthFile(LocalSourcePath);

impl PiAuthFile {
    fn from_environment() -> Result<Self, ExampleError> {
        let path = std::env::var("PI_AUTH_FILE")?;
        let document: serde_json::Value = serde_json::from_reader(std::fs::File::open(&path)?)?;
        let entries = document.as_object().ok_or(ExampleError::AuthFile)?;
        if entries.len() != 1
            || !entries
                .get("openai-codex")
                .is_some_and(serde_json::Value::is_object)
        {
            return Err(ExampleError::AuthFile);
        }
        Ok(Self(LocalSourcePath::new(path)?))
    }
}

const INSTALL: &str = r#"
set -eu
umask 077
mkdir -p /tmp/daytona-pi/auth /tmp/daytona-pi/work /tmp/daytona-pi/tool
timeout 210 npm install --prefix /tmp/daytona-pi/tool --ignore-scripts --no-audit --no-fund @earendil-works/pi-coding-agent@1.4.0
/tmp/daytona-pi/tool/node_modules/.bin/pi --version
"#;

const GENERATE: &str = r#"
set -eu
chmod 600 /tmp/daytona-pi/auth/auth.json
cd /tmp/daytona-pi/work
export PI_CODING_AGENT_DIR=/tmp/daytona-pi/auth
timeout 210 /tmp/daytona-pi/tool/node_modules/.bin/pi \
  --offline --print --provider openai-codex --model gpt-6-luna --thinking low \
  --no-session --no-extensions --no-mcp --no-skills --no-prompt-templates \
  --no-context-files --no-approve --tools write -- \
  'Create sum_even.py in the current directory using the write tool. It must define sum_even(numbers), returning the sum of the even integers in numbers. When run as a script, print sum_even(range(1, 11)), which is 30. Use only the Python standard library. Do not read any files or credentials. Create the file, do not only describe it.'
rm /tmp/daytona-pi/auth/auth.json
"#;

async fn generate(
    sandbox: &Sandbox<'_>,
    auth: &PiAuthFile,
    directory: &Path,
) -> Result<(), TaskError> {
    std::fs::write(
        directory.join("sandbox.json"),
        serde_json::to_string(sandbox.id())?,
    )?;

    // Install before sending the model credential to the sandbox.
    sandbox
        .shell(ShellScript::new(INSTALL)?)
        .execution_timeout(ExecutionTimeout::seconds(240)?)
        .checked_output()
        .await?;
    sandbox
        .upload(
            &auth.0,
            &SandboxPath::new("/tmp/daytona-pi/auth/auth.json")?,
        )
        .await?;
    sandbox
        .shell(ShellScript::new(GENERATE)?)
        .execution_timeout(ExecutionTimeout::seconds(240)?)
        .checked_output()
        .await?;

    // Check real execution separately from Pi's description of its work.
    let output = sandbox
        .shell(ShellScript::new(
            "cd /tmp/daytona-pi/work && timeout 10 python3 sum_even.py",
        )?)
        .execution_timeout(ExecutionTimeout::seconds(15)?)
        .checked_output()
        .await?;
    if output.text() != "30\n" {
        return Err(TaskError::OutputMismatch);
    }
    sandbox
        .download(
            &SandboxPath::new("/tmp/daytona-pi/work/sum_even.py")?,
            &LocalDestinationPath::new(directory.join("sum_even.py"))?,
        )
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ExampleError> {
    if !std::env::args().any(|argument| argument == "--run-live") {
        return Err(ExampleError::ConsentRequired);
    }
    let auth = PiAuthFile::from_environment()?;
    let key = ApiKey::new(std::env::var("DAYTONA_API_KEY")?)?;
    // The HTTP deadline must exceed each remote command's execution limit.
    let config = ClientConfig::cloud()?.request_timeout(RequestTimeout::seconds(270)?);
    let client = Daytona::with_config(Credentials::ApiKey(key), config)?;
    let scope = client
        .sandboxes()
        .ephemeral()
        .ttl(SandboxTtl::minutes(15)?)
        .readiness_timeout(WaitTimeout::seconds(120)?)
        .cleanup_timeout(WaitTimeout::seconds(90)?);

    // Save recovery data before creating the sandbox. This file contains no key.
    let directory = std::path::PathBuf::from(".plan")
        .join(format!("pi-{}", scope.reference().operation().as_str()));
    std::fs::create_dir_all(&directory)?;
    let mut reference = std::fs::File::create_new(directory.join("creation-reference.json"))?;
    reference.write_all(serde_json::to_string(scope.reference())?.as_bytes())?;
    reference.sync_all()?;
    writeln!(
        std::io::stdout().lock(),
        "Run directory: {}",
        directory.display()
    )?;

    scope
        .run(async |sandbox| generate(sandbox, &auth, &directory).await)
        .await?;

    // The scope already confirmed cleanup. Check again through a separate request.
    let id: SandboxId =
        serde_json::from_reader(std::fs::File::open(directory.join("sandbox.json"))?)?;
    match client.sandbox(id).observe().await? {
        None => {}
        Some(observation) if matches!(observation.state(), RemoteState::Destroyed) => {}
        Some(_) => return Err(ExampleError::StillPresent),
    }
    writeln!(
        std::io::stdout().lock(),
        "Program output: 30\nSaved: {}\nSandbox deletion confirmed.",
        directory.join("sum_even.py").display()
    )?;
    Ok(())
}
