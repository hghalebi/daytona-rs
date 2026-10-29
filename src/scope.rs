use crate::*;
use std::{error::Error, ops::AsyncFnOnce};

/// A scope that still requires a positive lifetime before execution.
pub struct NeedsTtl {
    reference: CreationReference,
}
/// A scope with a validated lifetime and an available run operation.
pub struct HasTtl {
    request: Create<Validated>,
}
/// A temporary-sandbox scope that owns cleanup on normal completion.
///
/// Set a TTL, then lend the sandbox to work. No cleanup runs when the future
/// is dropped or user code panics. See [getting started](crate::guide::getting_started).
pub struct Ephemeral<'client, State = NeedsTtl> {
    client: &'client Daytona,
    state: State,
    readiness: Option<WaitTimeout>,
    cleanup: Option<WaitTimeout>,
}
impl<'client> Ephemeral<'client> {
    pub(crate) fn new(client: &'client Daytona) -> Self {
        Self {
            client,
            state: NeedsTtl {
                reference: CreationReference::generate(),
            },
            readiness: None,
            cleanup: None,
        }
    }
    /// Moves the scope into its runnable state with a provider lifetime.
    ///
    /// A TTL does not confirm deletion; normal completion still performs cleanup.
    pub fn ttl(self, ttl: SandboxTtl) -> Ephemeral<'client, HasTtl> {
        Ephemeral {
            client: self.client,
            state: HasTtl {
                request: Create::scoped(ttl, self.state.reference),
            },
            readiness: self.readiness,
            cleanup: self.cleanup,
        }
    }
}
impl Ephemeral<'_, NeedsTtl> {
    /// Borrows correlation data to persist before execution.
    pub fn reference(&self) -> &CreationReference {
        &self.state.reference
    }
}
impl<State> Ephemeral<'_, State> {
    /// Sets the total readiness deadline. Default: 60 seconds.
    pub fn readiness_timeout(mut self, timeout: WaitTimeout) -> Self {
        self.readiness = Some(timeout);
        self
    }
    /// Sets the total deletion deadline. Default: 30 seconds.
    pub fn cleanup_timeout(mut self, timeout: WaitTimeout) -> Self {
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
#[derive(Debug, thiserror::Error)]
/// Work and cleanup failures reported independently.
///
/// Use both accessors when needed; the standard source chain exposes only one
/// branch. See [error handling](crate::guide::errors).
pub enum ScopeError<E: Error + 'static> {
    #[error(transparent)]
    /// Work did not begin. Inspect setup and any cleanup failure.
    Setup(#[from] ScopeSetupError),
    #[error("work failed after confirmed cleanup")]
    /// Work failed after cleanup was confirmed.
    Work(#[source] E),
    #[error("work succeeded but cleanup was not confirmed")]
    /// Work succeeded but cleanup was not confirmed. Its output was discarded.
    Cleanup(#[source] CleanupError),
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
impl<E: Error + 'static> ScopeError<E> {
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
            Self::Cleanup(e) | Self::Both { cleanup: e, .. } => Some(e),
            Self::Setup(e) => e.cleanup.as_deref(),
            Self::Work(_) => None,
        }
    }
}
#[derive(Debug)]
/// Work output and cleanup outcomes without discarding successful output.
///
/// Use [`Ephemeral::run_report`] when successful work remains useful even if
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
impl<T, E: Error + 'static> ScopeReport<T, E> {
    /// Converts a report to the convenience result, discarding output on cleanup failure.
    ///
    /// # Errors
    /// Returns [`ScopeError`] for work failure, cleanup failure, or both. Inspect the
    /// report directly first when you need successful output after failed cleanup.
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
    pub(crate) fn from_request(client: &'client Daytona, request: Create<Validated>) -> Self {
        Self {
            client,
            state: HasTtl { request },
            readiness: None,
            cleanup: None,
        }
    }
    /// Borrows the exact validated request's reference. Persist it before execution.
    pub fn reference(&self) -> &CreationReference {
        self.state.request.reference()
    }

    /// Creates a sandbox, lends it to work, then confirms deletion.
    ///
    /// Returned data must own what it needs after the scope ends. Use
    /// [`Self::run_report`] to retain output when cleanup fails.
    /// See [getting started](crate::guide::getting_started).
    ///
    /// # Errors
    /// Returns [`ScopeError`] for setup, work, cleanup, or combined failures.
    ///
    /// # Cancellation
    /// Dropping the future does not perform asynchronous cleanup. Persist the
    /// reference before awaiting and reconcile it after interruption. The callback
    /// has no imposed work deadline. No universal Send guarantee is made.
    pub async fn run<T, E, F>(self, work: F) -> Result<T, ScopeError<E>>
    where
        E: Error + 'static,
        F: for<'scope> AsyncFnOnce(&'scope Sandbox<'client>) -> Result<T, E>,
    {
        self.run_report(work).await?.into_result()
    }
    /// Runs scoped work and preserves its output alongside a cleanup failure.
    ///
    /// See [preserving output](crate::guide::errors::preserve_work).
    ///
    /// # Errors
    /// Returns [`ScopeSetupError`] if setup fails. Work and cleanup failures after
    /// setup are report variants, including both failures together.
    ///
    /// # Cancellation
    /// The same cancellation limits as [`Self::run`] apply. Save the reference first.
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
            let receipt = self.state.request.submit(self.client).await?;
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
