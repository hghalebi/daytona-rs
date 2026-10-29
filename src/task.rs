use crate::*;
use std::{
    error::Error,
    future::{Future, IntoFuture},
    pin::Pin,
};

/// Local input for a temporary sandbox, including its stable recovery reference.
#[derive(Debug)]
pub struct ScopeInput {
    source: ScopeSource,
    ttl: Option<Result<SandboxTtl, ValidationError>>,
    readiness: Option<WaitTimeout>,
    cleanup: Option<WaitTimeout>,
}
#[derive(Debug)]
enum ScopeSource {
    Default(CreationReference),
    Configured(Box<Create<Validated>>),
}
impl ScopeInput {
    fn new() -> Self {
        Self {
            source: ScopeSource::Default(CreationReference::generate()),
            ttl: Some(SandboxTtl::minutes(5)),
            readiness: None,
            cleanup: None,
        }
    }
    fn configured(request: Create<Validated>) -> Self {
        Self {
            source: ScopeSource::Configured(Box::new(request)),
            ttl: None,
            readiness: None,
            cleanup: None,
        }
    }
    /// Borrows the exact reference that will be submitted. Persist before awaiting.
    pub fn reference(&self) -> &CreationReference {
        match &self.source {
            ScopeSource::Default(reference) => reference,
            ScopeSource::Configured(request) => request.reference(),
        }
    }
    /// Replaces the lifetime without changing the recovery reference.
    pub fn ttl(mut self, ttl: SandboxTtl) -> Self {
        self.ttl = Some(Ok(ttl));
        self
    }
    /// Sets the readiness deadline. Default: 60 seconds.
    pub fn readiness_timeout(mut self, timeout: WaitTimeout) -> Self {
        self.readiness = Some(timeout);
        self
    }
    /// Sets the deletion-confirmation deadline. Default: 30 seconds.
    pub fn cleanup_timeout(mut self, timeout: WaitTimeout) -> Self {
        self.cleanup = Some(timeout);
        self
    }
    fn validate(mut self) -> Result<ValidatedScope, ScopePreparationError> {
        let ttl = match self.ttl.take() {
            Some(Ok(ttl)) => Some(ttl),
            Some(Err(cause)) => {
                self.ttl = Some(Err(ValidationError::CorrectionRequired { field: "TTL" }));
                return Err(ScopePreparationError { input: self, cause });
            }
            None => None,
        };
        let request = match self.source {
            ScopeSource::Default(reference) => match ttl {
                Some(ttl) => Create::scoped(ttl, reference),
                None => {
                    return Err(ScopePreparationError {
                        input: Self {
                            source: ScopeSource::Default(reference),
                            ttl: None,
                            readiness: self.readiness,
                            cleanup: self.cleanup,
                        },
                        cause: ValidationError::MissingTtl,
                    });
                }
            },
            ScopeSource::Configured(request) => match ttl {
                Some(ttl) => (*request).with_ttl(ttl),
                None => *request,
            },
        };
        Ok(ValidatedScope {
            request,
            readiness: self.readiness,
            cleanup: self.cleanup,
        })
    }
}
struct ValidatedScope {
    request: Create<Validated>,
    readiness: Option<WaitTimeout>,
    cleanup: Option<WaitTimeout>,
}
impl ValidatedScope {
    fn runner(self, client: &Daytona) -> crate::scope::ScopeRunner<'_> {
        let mut scope = crate::scope::ScopeRunner::from_request(client, self.request);
        if let Some(timeout) = self.readiness {
            scope = scope.readiness_timeout(timeout);
        }
        if let Some(timeout) = self.cleanup {
            scope = scope.cleanup_timeout(timeout);
        }
        scope
    }
}
/// Invalid scope configuration retained for correction.
#[derive(Debug, thiserror::Error)]
#[error("scope preparation failed: {cause}")]
pub struct ScopePreparationError {
    input: ScopeInput,
    #[source]
    cause: ValidationError,
}
impl ScopePreparationError {
    /// Returns the complete scope input and violation.
    pub fn into_parts(self) -> (ScopeInput, ValidationError) {
        (self.input, self.cause)
    }
    /// Borrows the original creation reference.
    pub fn reference(&self) -> &CreationReference {
        self.input.reference()
    }
}
/// Local preparation failed before any remote effect.
#[derive(Debug, thiserror::Error)]
pub enum PreparationError {
    /// Scope input failed; an optional command draft remains available.
    #[error("scope input is invalid")]
    Scope {
        #[source]
        /// Scope input and validation cause.
        error: Box<ScopePreparationError>,
        /// Command draft when preparing a single-command workflow.
        command: Option<Box<CommandInput>>,
    },
    /// Command input failed; its scope input remains available.
    #[error("command input is invalid")]
    Command {
        #[source]
        /// Complete command draft and validation cause.
        error: Box<CommandPreparationError>,
        /// Complete scope input with the original reference.
        scope: Box<ScopeInput>,
    },
}
/// The error returned by one command in a temporary sandbox.
pub type RunError = ScopeError<CheckedOutput, ExecutionError>;

/// A command in a temporary sandbox. Await checks its exit and confirms deletion.
///
/// Defaults: TTL 5 minutes, execution 30 seconds, readiness 60 seconds, cleanup
/// 30 seconds. Client request/capture limits still apply. Dropping a polled task
/// does not clean up asynchronously. Persist reference() before await for recovery.
/// Await allocates one boxed future; execute() uses the underlying async method.
#[must_use = "tasks do nothing until awaited"]
pub struct Run<'client> {
    client: &'client Daytona,
    scope: ScopeInput,
    command: CommandInput,
}
impl<'client> Run<'client> {
    /// Borrows the stable creation reference before execution.
    pub fn reference(&self) -> &CreationReference {
        self.scope.reference()
    }
    /// Sets the provider lifetime without changing the reference.
    pub fn ttl(mut self, ttl: SandboxTtl) -> Self {
        self.scope = self.scope.ttl(ttl);
        self
    }
    /// Sets readiness waiting time.
    pub fn readiness_timeout(mut self, timeout: WaitTimeout) -> Self {
        self.scope = self.scope.readiness_timeout(timeout);
        self
    }
    /// Sets deletion-confirmation waiting time.
    pub fn cleanup_timeout(mut self, timeout: WaitTimeout) -> Self {
        self.scope = self.scope.cleanup_timeout(timeout);
        self
    }
    /// Sets execution and request limits. Last timing setter wins.
    pub fn execution_timeout(mut self, timeout: ExecutionTimeout) -> Self {
        self.command = self.command.execution_timeout(timeout);
        self
    }
    /// Sets independent execution and HTTP limits. The client cap still applies.
    pub fn budget(mut self, budget: CommandBudget) -> Self {
        self.command = self.command.budget(budget);
        self
    }
    /// Sets the command working directory.
    pub fn cwd(mut self, path: SandboxPath) -> Self {
        self.command = self.command.cwd(path);
        self
    }
    #[doc(hidden)]
    pub fn __ttl(mut self, ttl: Result<SandboxTtl, ValidationError>) -> Self {
        self.scope.ttl = Some(ttl);
        self
    }
    #[doc(hidden)]
    pub fn __execution_timeout(
        mut self,
        timeout: Result<ExecutionTimeout, ValidationError>,
    ) -> Self {
        self.command.timing = timeout.map(crate::execution::CommandTiming::Coupled);
        self
    }
    /// Runs once and preserves successful output if cleanup fails.
    pub async fn execute(self) -> Result<CheckedOutput, RunError> {
        let command = match self.command.validate() {
            Ok(command) => command,
            Err(error) => {
                return Err(PreparationError::Command {
                    error: Box::new(error),
                    scope: Box::new(self.scope),
                }
                .into());
            }
        };
        let scope = match self.scope.validate() {
            Ok(scope) => scope,
            Err(error) => {
                return Err(PreparationError::Scope {
                    error: Box::new(error),
                    command: Some(Box::new(CommandInput {
                        script: command.script.into(),
                        timing: Ok(command.timing),
                        cwd: command.cwd,
                    })),
                }
                .into());
            }
        };
        scope
            .runner(self.client)
            .run_report(async move |sandbox| {
                use crate::execution::ShellExecutor;
                sandbox.execute(&command).await?.checked()
            })
            .await?
            .into_result()
    }
}
impl<'client> IntoFuture for Run<'client> {
    type Output = Result<CheckedOutput, RunError>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + 'client>>;
    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}

/// Scoped work borrowing one temporary sandbox and returning owned output.
///
/// Await confirms deletion before Ok. All work/cleanup outcomes retain their
/// data. There is no implicit callback deadline or universal Send guarantee.
/// Save reference() before await; cancellation does not perform async cleanup.
#[must_use = "scopes do nothing until awaited"]
pub struct ScopeTask<'client, F> {
    client: &'client Daytona,
    input: ScopeInput,
    work: F,
}
impl<'client, F> ScopeTask<'client, F> {
    /// Borrows the exact creation reference for recovery persistence.
    pub fn reference(&self) -> &CreationReference {
        self.input.reference()
    }
    /// Sets lifetime without changing source policies or reference.
    pub fn ttl(mut self, ttl: SandboxTtl) -> Self {
        self.input = self.input.ttl(ttl);
        self
    }
    /// Sets the readiness deadline.
    pub fn readiness_timeout(mut self, timeout: WaitTimeout) -> Self {
        self.input = self.input.readiness_timeout(timeout);
        self
    }
    /// Sets deletion-confirmation waiting time.
    pub fn cleanup_timeout(mut self, timeout: WaitTimeout) -> Self {
        self.input = self.input.cleanup_timeout(timeout);
        self
    }
    #[doc(hidden)]
    pub fn __ttl(mut self, ttl: Result<SandboxTtl, ValidationError>) -> Self {
        self.input.ttl = Some(ttl);
        self
    }
    /// Runs work and returns the independent work/cleanup outcomes.
    pub async fn report<T, E>(self) -> Result<ScopeReport<T, E>, ScopeError<T, E>>
    where
        F: for<'scope> AsyncFnOnce(&'scope Sandbox<'client>) -> Result<T, E>,
        E: Error + 'static,
    {
        let input = self
            .input
            .validate()
            .map_err(|error| PreparationError::Scope {
                error: Box::new(error),
                command: None,
            })?;
        Ok(input.runner(self.client).run_report(self.work).await?)
    }
    /// Runs work and confirms deletion, retaining successful output on cleanup failure.
    pub async fn execute<T, E>(self) -> Result<T, ScopeError<T, E>>
    where
        F: for<'scope> AsyncFnOnce(&'scope Sandbox<'client>) -> Result<T, E>,
        E: Error + 'static,
    {
        self.report().await?.into_result()
    }
}
impl<'client, F, T, E> IntoFuture for ScopeTask<'client, F>
where
    F: for<'scope> AsyncFnOnce(&'scope Sandbox<'client>) -> Result<T, E> + 'client,
    T: 'client,
    E: Error + 'static,
{
    type Output = Result<T, ScopeError<T, E>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + 'client>>;
    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}
impl Daytona {
    /// Prepares one checked command in a temporary sandbox. Await also confirms deletion.
    ///
    /// Accepts text or ShellScript. Local validation happens before creation.
    pub fn run(&self, script: impl Into<ScriptInput>) -> Run<'_> {
        self.resume_run(ScopeInput::new(), CommandInput::new(script))
    }
    /// Resumes corrected owned inputs. No automatic retry or replay is performed.
    pub fn resume_run(&self, scope: ScopeInput, command: CommandInput) -> Run<'_> {
        Run {
            client: self,
            scope,
            command,
        }
    }
    /// Lends a temporary sandbox to work, then confirms deletion on ordinary completion.
    pub fn scope<'client, F, T, E>(&'client self, work: F) -> ScopeTask<'client, F>
    where
        F: for<'scope> AsyncFnOnce(&'scope Sandbox<'client>) -> Result<T, E>,
        E: Error + 'static,
    {
        ScopeTask {
            client: self,
            input: ScopeInput::new(),
            work,
        }
    }
    /// Runs scoped work using a validated request, preserving its reference and policies.
    pub fn scope_from<'client, F, T, E>(
        &'client self,
        request: Create<Validated>,
        work: F,
    ) -> ScopeTask<'client, F>
    where
        F: for<'scope> AsyncFnOnce(&'scope Sandbox<'client>) -> Result<T, E>,
        E: Error + 'static,
    {
        ScopeTask {
            client: self,
            input: ScopeInput::configured(request),
            work,
        }
    }
    /// Resumes corrected scope input with explicitly supplied work.
    pub fn resume_scope<'client, F, T, E>(
        &'client self,
        input: ScopeInput,
        work: F,
    ) -> ScopeTask<'client, F>
    where
        F: for<'scope> AsyncFnOnce(&'scope Sandbox<'client>) -> Result<T, E>,
        E: Error + 'static,
    {
        ScopeTask {
            client: self,
            input,
            work,
        }
    }
}
