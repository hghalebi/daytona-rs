use crate::*;
use std::{error::Error, fmt, ops::AsyncFnOnce};

pub(crate) struct ScopeRunner<'client> {
    client: &'client Daytona,
    request: Create<Validated>,
    readiness: Option<WaitTimeout>,
    cleanup: Option<WaitTimeout>,
}
impl<'client> ScopeRunner<'client> {
    pub(crate) fn from_request(client: &'client Daytona, request: Create<Validated>) -> Self {
        Self {
            client,
            request,
            readiness: None,
            cleanup: None,
        }
    }
    pub(crate) fn readiness_timeout(mut self, timeout: WaitTimeout) -> Self {
        self.readiness = Some(timeout);
        self
    }
    pub(crate) fn cleanup_timeout(mut self, timeout: WaitTimeout) -> Self {
        self.cleanup = Some(timeout);
        self
    }
}
#[derive(Debug, thiserror::Error)]
/// A failure before scoped work began. See [`ScopeSetupError`] for cleanup.
pub enum SetupError {
    #[error(transparent)]
    /// A local domain value could not be constructed.
    Validation(#[from] ValidationError),
    #[error(transparent)]
    /// The creation specification could not be validated.
    CreateValidation(#[from] CreateValidationError),
    #[error(transparent)]
    /// Submission failed; inspect its retained state or recovery reference.
    Create(#[from] CreateError),
    #[error("readiness deadline elapsed")]
    /// Readiness was not observed before the deadline.
    ReadinessDeadline,
    #[error("sandbox reached terminal state before readiness")]
    /// The sandbox reached a terminal state before readiness.
    ReadinessTerminal,
    #[error("readiness observation failed")]
    /// An observation failed before readiness.
    ReadinessObservation(#[source] OperationError),
}
#[derive(Debug, thiserror::Error)]
#[error("scope setup failed")]
/// Setup failure plus any failure to clean up an acknowledged sandbox.
///
/// If creation was uncertain, inspect the creation error and reconcile its reference.
pub struct ScopeSetupError {
    #[source]
    /// The failure that prevented work from starting.
    pub cause: SetupError,
    /// A cleanup failure after setup, if cleanup was attempted and failed.
    pub cleanup: Option<Box<CleanupError>>,
}
#[derive(thiserror::Error)]
/// Work and cleanup failures reported independently.
///
/// Use both accessors when needed; the standard source chain exposes only one
/// branch. See [error handling](crate::guide::errors).
pub enum ScopeError<T, E: Error + 'static> {
    #[error(transparent)]
    /// Local preparation failed. All owned input remains available for correction.
    Preparation(#[from] PreparationError),
    #[error(transparent)]
    /// Work did not begin. Inspect setup and any cleanup failure.
    Setup(#[from] ScopeSetupError),
    #[error("work failed after confirmed cleanup")]
    /// Work failed after cleanup was confirmed.
    Work(#[source] E),
    #[error("work succeeded but cleanup was not confirmed")]
    /// Work succeeded but cleanup was not confirmed. Successful output is retained.
    Cleanup {
        /// Owned successful output, redacted from diagnostics.
        output: T,
        #[source]
        /// Failure to confirm deletion.
        cleanup: CleanupError,
    },
    #[error("work and cleanup both failed")]
    /// Work and cleanup both failed; retain and inspect each.
    Both {
        #[source]
        /// Failure returned by the work callback.
        work: E,
        /// Failure to confirm deletion.
        cleanup: CleanupError,
    },
}
impl<T, E: Error + 'static> ScopeError<T, E> {
    /// Borrows successful output when cleanup failed.
    pub fn successful_output(&self) -> Option<&T> {
        match self {
            Self::Cleanup { output, .. } => Some(output),
            _ => None,
        }
    }
    /// Borrows the work failure, if work began and failed.
    pub fn work_error(&self) -> Option<&E> {
        match self {
            Self::Work(e) | Self::Both { work: e, .. } => Some(e),
            _ => None,
        }
    }
    /// Borrows a cleanup failure, including cleanup after failed readiness.
    pub fn cleanup_error(&self) -> Option<&CleanupError> {
        match self {
            Self::Cleanup { cleanup: e, .. } | Self::Both { cleanup: e, .. } => Some(e),
            Self::Setup(e) => e.cleanup.as_deref(),
            Self::Work(_) | Self::Preparation(_) => None,
        }
    }
}
impl<T, E: Error + 'static> fmt::Debug for ScopeError<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preparation(error) => f.debug_tuple("Preparation").field(error).finish(),
            Self::Setup(error) => f.debug_tuple("Setup").field(error).finish(),
            Self::Work(_) => f.write_str("Work([REDACTED])"),
            Self::Cleanup { cleanup, .. } => f
                .debug_struct("Cleanup")
                .field("output", &"[REDACTED]")
                .field("cleanup", cleanup)
                .finish(),
            Self::Both { cleanup, .. } => f
                .debug_struct("Both")
                .field("work", &"[REDACTED]")
                .field("cleanup", cleanup)
                .finish(),
        }
    }
}
/// Work output and cleanup outcomes without discarding successful output.
///
/// Use [`ScopeTask::report`] when successful work remains useful even if
/// cleanup fails. See [preserving output](crate::guide::errors::preserve_work).
pub enum ScopeReport<T, E> {
    /// Work succeeded and cleanup was confirmed.
    Success(T),
    /// Work failed and cleanup was confirmed.
    WorkFailed(E),
    /// Work succeeded, and its output is retained with the cleanup failure.
    CleanupFailed {
        /// Successful owned output retained for the caller.
        output: T,
        /// Failure to confirm deletion.
        cleanup: CleanupError,
    },
    /// Both work and cleanup failed.
    BothFailed {
        /// Failure returned by the work callback.
        work: E,
        /// Failure to confirm deletion.
        cleanup: CleanupError,
    },
}
impl<T, E> fmt::Debug for ScopeReport<T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Success(_) => "Success([REDACTED])",
            Self::WorkFailed(_) => "WorkFailed([REDACTED])",
            Self::CleanupFailed { .. } => "CleanupFailed([REDACTED])",
            Self::BothFailed { .. } => "BothFailed([REDACTED])",
        })
    }
}
impl<T, E: Error + 'static> ScopeReport<T, E> {
    /// Converts a report to a result, retaining successful output in a cleanup error.
    ///
    /// # Errors
    /// Returns [`ScopeError`] for work failure, cleanup failure, or both. Inspect the
    /// successful_output accessor or match the error to recover owned output.
    pub fn into_result(self) -> Result<T, ScopeError<T, E>> {
        match self {
            Self::Success(output) => Ok(output),
            Self::WorkFailed(work) => Err(ScopeError::Work(work)),
            Self::CleanupFailed { output, cleanup } => Err(ScopeError::Cleanup { output, cleanup }),
            Self::BothFailed { work, cleanup } => Err(ScopeError::Both { work, cleanup }),
        }
    }
}
impl<'client> ScopeRunner<'client> {
    pub(crate) async fn run_report<T, E, F>(
        self,
        work: F,
    ) -> Result<ScopeReport<T, E>, ScopeSetupError>
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
            let receipt = self.request.submit(self.client).await?;
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
