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
    #[error("Pi work or credential removal failed (statuses retained)")]
    Generation { work: ExitCode, removal: ExitCode },
    #[error("Pi credential removal could not be established")]
    RemovalUnknown,
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
root=${PI_TASK_ROOT:-/tmp/daytona-pi}
finish() {
  work=$?
  trap - EXIT
  removal=0
  rm -f "$root/auth/auth.json" || removal=$?
  printf '\nDAYTONA_PI_STATUS:{"work":%s,"removal":%s}\n' "$work" "$removal"
  if [ "$work" -ne 0 ]; then exit "$work"; fi
  exit "$removal"
}
trap finish EXIT
chmod 600 "$root/auth/auth.json"
cd "$root/work"
export PI_CODING_AGENT_DIR="$root/auth"
timeout 210 "$root/tool/node_modules/.bin/pi" \
  --offline --print --provider openai-codex --model gpt-6-luna --thinking low \
  --no-session --no-extensions --no-mcp --no-skills --no-prompt-templates \
  --no-context-files --no-approve --tools write -- \
  'Create sum_even.py in the current directory using the write tool. It must define sum_even(numbers), returning the sum of the even integers in numbers. When run as a script, print sum_even(range(1, 11)), which is 30. Use only the Python standard library. Do not read any files or credentials. Create the file, do not only describe it.'
"#;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct GenerationStatus {
    work: ExitCode,
    removal: ExitCode,
}
fn check_generation(text: &str) -> Result<(), TaskError> {
    let record = text
        .lines()
        .last()
        .and_then(|line| line.strip_prefix("DAYTONA_PI_STATUS:"))
        .ok_or(TaskError::RemovalUnknown)?;
    let status: GenerationStatus =
        serde_json::from_str(record).map_err(|_| TaskError::RemovalUnknown)?;
    if status.work.is_success() && status.removal.is_success() {
        Ok(())
    } else {
        Err(TaskError::Generation {
            work: status.work,
            removal: status.removal,
        })
    }
}

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
    let generation = sandbox
        .shell(ShellScript::new(GENERATE)?)
        .execution_timeout(ExecutionTimeout::seconds(240)?)
        .output()
        .await?;
    check_generation(generation.text())?;
    generation.checked()?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[derive(Debug, Error)]
    enum TestError {
        #[error(transparent)]
        Io(#[from] std::io::Error),
        #[error("credential removal contract failed")]
        Contract,
    }
    fn executable(path: &Path, source: &str) -> Result<(), TestError> {
        std::fs::write(path, source)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
        Ok(())
    }
    #[test]
    fn generation_removal_preserves_both_outcomes() -> Result<(), TestError> {
        for (work, removal) in [(0, 0), (7, 0), (0, 9), (7, 9), (124, 0)] {
            let root = std::env::temp_dir().join(format!("pi-cleanup-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(root.join("auth"))?;
            std::fs::create_dir_all(root.join("work"))?;
            std::fs::create_dir_all(root.join("bin"))?;
            std::fs::create_dir_all(root.join("tool/node_modules/.bin"))?;
            let result = (|| -> Result<(), TestError> {
                std::fs::write(root.join("auth/auth.json"), "synthetic")?;
                executable(
                    &root.join("tool/node_modules/.bin/pi"),
                    &format!("#!/bin/sh\nexit {work}\n"),
                )?;
                executable(&root.join("bin/timeout"), "#!/bin/sh\nshift\nexec \"$@\"\n")?;
                if removal != 0 {
                    executable(
                        &root.join("bin/rm"),
                        &format!("#!/bin/sh\nexit {removal}\n"),
                    )?;
                }
                let output = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(GENERATE)
                    .env("PI_TASK_ROOT", &root)
                    .env(
                        "PATH",
                        format!("{}:/usr/bin:/bin", root.join("bin").display()),
                    )
                    .output()?;
                let text = std::str::from_utf8(&output.stdout).map_err(|_| TestError::Contract)?;
                match check_generation(text) {
                    Ok(()) if work == 0 && removal == 0 => {}
                    Err(TaskError::Generation {
                        work: w,
                        removal: r,
                    }) if w.get() == work && r.get() == removal => {}
                    _ => return Err(TestError::Contract),
                }
                if root.join("auth/auth.json").exists() != (removal != 0) {
                    return Err(TestError::Contract);
                }
                Ok(())
            })();
            let cleanup = std::fs::remove_dir_all(&root);
            cleanup?;
            result?;
        }
        if !matches!(
            check_generation("no record"),
            Err(TaskError::RemovalUnknown)
        ) {
            return Err(TestError::Contract);
        }
        Ok(())
    }
}
