use crate::*;
use std::{error::Error, ops::AsyncFnOnce};

pub struct NeedsTtl;
pub struct HasTtl {
    ttl: SandboxTtl,
}
pub struct Ephemeral<'client, State = NeedsTtl> {
    client: &'client Daytona,
    reference: CreationReference,
    state: State,
    readiness: Option<WaitTimeout>,
    cleanup: Option<WaitTimeout>,
}
impl<'client> Ephemeral<'client> {
    pub(crate) fn new(client: &'client Daytona) -> Self {
        Self {
            client,
            reference: CreationReference::generate(),
            state: NeedsTtl,
            readiness: None,
            cleanup: None,
        }
    }
    pub fn ttl(self, ttl: SandboxTtl) -> Ephemeral<'client, HasTtl> {
        Ephemeral {
            client: self.client,
            reference: self.reference,
            state: HasTtl { ttl },
            readiness: self.readiness,
            cleanup: self.cleanup,
        }
    }
}
impl<State> Ephemeral<'_, State> {
    /// Serialize before running when cancellation recovery is required.
    pub fn reference(&self) -> &CreationReference {
        &self.reference
    }
    pub fn readiness_timeout(mut self, timeout: WaitTimeout) -> Self {
        self.readiness = Some(timeout);
        self
    }
    pub fn cleanup_timeout(mut self, timeout: WaitTimeout) -> Self {
        self.cleanup = Some(timeout);
        self
    }
}
#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    CreateValidation(#[from] CreateValidationError),
    #[error(transparent)]
    Create(#[from] CreateError),
    #[error("readiness deadline elapsed")]
    ReadinessDeadline,
    #[error("sandbox reached terminal state before readiness")]
    ReadinessTerminal,
    #[error("readiness observation failed")]
    ReadinessObservation(#[source] OperationError),
}
#[derive(Debug, thiserror::Error)]
#[error("scope setup failed")]
pub struct ScopeSetupError {
    #[source]
    pub cause: SetupError,
    pub cleanup: Option<Box<CleanupError>>,
}
#[derive(Debug, thiserror::Error)]
pub enum ScopeError<E: Error + 'static> {
    #[error(transparent)]
    Setup(#[from] ScopeSetupError),
    #[error("work failed after confirmed cleanup")]
    Work(#[source] E),
    #[error("work succeeded but cleanup was not confirmed")]
    Cleanup(#[source] CleanupError),
    #[error("work and cleanup both failed")]
    Both {
        #[source]
        work: E,
        cleanup: CleanupError,
    },
}
impl<E: Error + 'static> ScopeError<E> {
    pub fn work_error(&self) -> Option<&E> {
        match self {
            Self::Work(e) | Self::Both { work: e, .. } => Some(e),
            _ => None,
        }
    }
    pub fn cleanup_error(&self) -> Option<&CleanupError> {
        match self {
            Self::Cleanup(e) | Self::Both { cleanup: e, .. } => Some(e),
            Self::Setup(e) => e.cleanup.as_deref(),
            Self::Work(_) => None,
        }
    }
}
/// Successful output remains available even when cleanup fails.
#[derive(Debug)]
pub enum ScopeReport<T, E> {
    Success(T),
    WorkFailed(E),
    CleanupFailed { output: T, cleanup: CleanupError },
    BothFailed { work: E, cleanup: CleanupError },
}
impl<T, E: Error + 'static> ScopeReport<T, E> {
    pub fn into_result(self) -> Result<T, ScopeError<E>> {
        match self {
            Self::Success(output) => Ok(output),
            Self::WorkFailed(work) => Err(ScopeError::Work(work)),
            Self::CleanupFailed { cleanup, .. } => Err(ScopeError::Cleanup(cleanup)),
            Self::BothFailed { work, cleanup } => Err(ScopeError::Both { work, cleanup }),
        }
    }
}
impl<'client> Ephemeral<'client, HasTtl> {
    pub async fn run<T, E, F>(self, work: F) -> Result<T, ScopeError<E>>
    where
        E: Error + 'static,
        F: for<'scope> AsyncFnOnce(&'scope Sandbox<'client>) -> Result<T, E>,
    {
        self.run_report(work).await?.into_result()
    }
    pub async fn run_report<T, E, F>(self, work: F) -> Result<ScopeReport<T, E>, ScopeSetupError>
    where
        F: for<'scope> AsyncFnOnce(&'scope Sandbox<'client>) -> Result<T, E>,
    {
        let setup = async {
            let readiness = match self.readiness {
                Some(t) => t,
                None => WaitTimeout::seconds(60)?,
            };
            let cleanup = match self.cleanup {
                Some(t) => t,
                None => WaitTimeout::seconds(30)?,
            };
            let request = Create::draft()
                .ttl(self.state.ttl)
                .validate()?
                .with_reference(self.reference);
            let receipt = request.submit(self.client).await?;
            Ok::<_, SetupError>((receipt, readiness, cleanup))
        }
        .await;
        let (receipt, readiness, cleanup) = setup.map_err(|cause| ScopeSetupError {
            cause,
            cleanup: None,
        })?;
        let ready = match receipt.wait(self.client, readiness).await {
            Ok(ready) => ready,
            Err(failure) => {
                let (receipt, cause) = match failure {
                    WaitError::Deadline { receipt } => (receipt, SetupError::ReadinessDeadline),
                    WaitError::Terminal { receipt } => (receipt, SetupError::ReadinessTerminal),
                    WaitError::Observation { receipt, cause } => {
                        (receipt, SetupError::ReadinessObservation(cause))
                    }
                };
                let cleanup = self
                    .client
                    .sandbox(receipt.into_id())
                    .delete(cleanup)
                    .await
                    .err()
                    .map(Box::new);
                return Err(ScopeSetupError { cause, cleanup });
            }
        };
        let sandbox = ready.into_sandbox(self.client);
        let result = work(&sandbox).await;
        let cleanup = sandbox.delete(cleanup).await;
        Ok(match (result, cleanup) {
            (Ok(output), Ok(_)) => ScopeReport::Success(output),
            (Err(work), Ok(_)) => ScopeReport::WorkFailed(work),
            (Ok(output), Err(cleanup)) => ScopeReport::CleanupFailed { output, cleanup },
            (Err(work), Err(cleanup)) => ScopeReport::BothFailed { work, cleanup },
        })
    }
}
