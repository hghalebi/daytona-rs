//! Persist intent before creation, then reconcile it in the same trusted context.
use daytona::*;
use serde::{Deserialize, Serialize};
use std::{io::Write, path::PathBuf, time::SystemTime};
use thiserror::Error;

#[derive(Debug, Error)]
enum ExampleError {
    #[error("usage: recover_creation RECORD [--start --run-live | --delete SANDBOX_ID]")]
    Usage,
    #[error("recovery organization differs from configured organization")]
    Context,
    #[error("found identity differs from the identity explicitly approved for deletion")]
    Identity,
    #[error(transparent)]
    Environment(#[from] std::env::VarError),
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
    #[error(transparent)]
    Lookup(#[from] OperationError),
    #[error(transparent)]
    Cleanup(#[from] CleanupError),
    #[error(transparent)]
    Scope(#[from] ScopeError<ExecutionError>),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("recovery record is invalid or uses an unsupported schema")]
    Record,
    #[error("read-only recovery deadline elapsed; outcome remains unresolved")]
    Deadline,
}
#[derive(Serialize, Deserialize)]
enum Schema {
    V1,
}
// This lesson supports cloud only. Saved data cannot choose credential recipients.
#[derive(Serialize, Deserialize)]
enum Environment {
    DaytonaCloud,
}
#[derive(Serialize, Deserialize)]
struct LocalIntentTime(SystemTime);
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryRecord<R> {
    schema: Schema,
    environment: Environment,
    organization: OrganizationId,
    reference: R,
    intended_ttl: SandboxTtl,
    locally_recorded_at: LocalIntentTime,
}
enum Action {
    Observe,
    Start,
    Delete(SandboxId),
}
fn action() -> Result<(PathBuf, Action), ExampleError> {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(args.next().ok_or(ExampleError::Usage)?);
    let action = match args.next().as_deref() {
        None => Action::Observe,
        Some("--start") if args.next().as_deref() == Some("--run-live") => Action::Start,
        Some("--delete") => {
            Action::Delete(SandboxId::new(args.next().ok_or(ExampleError::Usage)?)?)
        }
        _ => return Err(ExampleError::Usage),
    };
    if args.next().is_some() {
        return Err(ExampleError::Usage);
    }
    Ok((path, action))
}
fn decode(
    bytes: &[u8],
    organization: &OrganizationId,
) -> Result<RecoveryRecord<CreationReference>, ExampleError> {
    let record: RecoveryRecord<CreationReference> =
        serde_json::from_slice(bytes).map_err(|_| ExampleError::Record)?;
    if &record.organization != organization {
        return Err(ExampleError::Context);
    }
    Ok(record)
}
#[tokio::main]
async fn main() -> Result<(), ExampleError> {
    let (path, action) = action()?;
    let organization = OrganizationId::new(std::env::var("DAYTONA_ORGANIZATION_ID")?)?;
    // Validate saved context before constructing a client. The current credential
    // must belong to this organization; the label itself grants no authority.
    let saved = match action {
        Action::Start => None,
        _ => Some(decode(&std::fs::read(&path)?, &organization)?),
    };
    let client = Daytona::new(Credentials::ApiKey(ApiKey::new(std::env::var(
        "DAYTONA_API_KEY",
    )?)?))?;
    if let Action::Start = action {
        let ttl = SandboxTtl::minutes(5)?;
        let scope = client.sandboxes().ephemeral().ttl(SandboxTtl::minutes(5)?);
        let record = RecoveryRecord {
            schema: Schema::V1,
            environment: Environment::DaytonaCloud,
            organization,
            reference: scope.reference(),
            intended_ttl: ttl,
            locally_recorded_at: LocalIntentTime(SystemTime::now()),
        };
        let mut file = std::fs::File::create_new(path)?;
        serde_json::to_writer(&mut file, &record).map_err(|_| ExampleError::Record)?;
        file.sync_all()?; // Failure here prevents creation.
        let script = ShellScript::new("printf 'recovery lesson\\n'")?;
        let output = scope
            .run(async move |sandbox| sandbox.shell(script).checked_output().await)
            .await?;
        write!(std::io::stdout().lock(), "{}", output.text())?;
        return Ok(());
    }
    let record = saved.ok_or(ExampleError::Record)?;
    let reconciliation = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        for _ in 0..3 {
            match record.reference.reconcile(&client).await? {
                Reconciliation::Found(receipt) => {
                    return Ok::<_, OperationError>(Reconciliation::Found(receipt));
                }
                Reconciliation::Unresolved => {
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await
                }
            }
        }
        Ok(Reconciliation::Unresolved)
    })
    .await
    .map_err(|_| ExampleError::Deadline)??;
    match reconciliation {
        Reconciliation::Found(receipt) => {
            writeln!(
                std::io::stdout().lock(),
                "Found sandbox: {}",
                receipt.sandbox_id().as_str()
            )?;
            if let Action::Delete(approved) = action {
                if receipt.sandbox_id() != &approved {
                    return Err(ExampleError::Identity);
                }
                client
                    .sandbox(receipt.into_id())
                    .delete(WaitTimeout::seconds(30)?)
                    .await?;
                writeln!(std::io::stdout().lock(), "Deletion confirmed")?;
            }
        }
        Reconciliation::Unresolved => writeln!(
            std::io::stdout().lock(),
            "Unresolved; do not replay creation"
        )?,
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn records_reject_context_and_schema_changes() -> Result<(), ExampleError> {
        let reference = Create::draft()
            .ttl(SandboxTtl::minutes(5)?)
            .validate()
            .map_err(|_| ExampleError::Record)?;
        let record = RecoveryRecord {
            schema: Schema::V1,
            environment: Environment::DaytonaCloud,
            organization: OrganizationId::new("org-a")?,
            reference: reference.reference(),
            intended_ttl: SandboxTtl::minutes(5)?,
            locally_recorded_at: LocalIntentTime(SystemTime::now()),
        };
        let bytes = serde_json::to_string(&record).map_err(|_| ExampleError::Record)?;
        decode(bytes.as_bytes(), &OrganizationId::new("org-a")?)?;
        if !matches!(
            decode(bytes.as_bytes(), &OrganizationId::new("org-b")?),
            Err(ExampleError::Context)
        ) {
            return Err(ExampleError::Record);
        }
        let source = bytes;
        for invalid in [
            source.replace("V1", "V99"),
            source.replace("DaytonaCloud", "https://untrusted.invalid"),
        ] {
            if !matches!(
                decode(invalid.as_bytes(), &OrganizationId::new("org-a")?),
                Err(ExampleError::Record)
            ) {
                return Err(ExampleError::Record);
            }
        }
        Ok(())
    }
}
