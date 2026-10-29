use crate::*;
use std::{
    fmt,
    future::{Future, IntoFuture},
    pin::Pin,
};

/// An owned command draft. Validation occurs before any execution request.
#[derive(Debug)]
pub struct CommandInput {
    pub(crate) script: ScriptInput,
    pub(crate) timing: Result<crate::execution::CommandTiming, ValidationError>,
    pub(crate) cwd: Option<SandboxPath>,
}
impl CommandInput {
    pub(crate) fn new(script: impl Into<ScriptInput>) -> Self {
        Self {
            script: script.into(),
            timing: ExecutionTimeout::seconds(30).map(crate::execution::CommandTiming::Coupled),
            cwd: None,
        }
    }
    /// Replaces script input without copying owned values.
    pub fn script(mut self, script: impl Into<ScriptInput>) -> Self {
        self.script = script.into();
        self
    }
    /// Replaces the execution and HTTP request limit.
    pub fn execution_timeout(mut self, timeout: ExecutionTimeout) -> Self {
        self.timing = Ok(crate::execution::CommandTiming::Coupled(timeout));
        self
    }
    /// Sets independent execution and HTTP request limits.
    pub fn budget(mut self, budget: CommandBudget) -> Self {
        self.timing = Ok(crate::execution::CommandTiming::Separate(budget));
        self
    }
    /// Sets the remote working directory.
    pub fn cwd(mut self, path: SandboxPath) -> Self {
        self.cwd = Some(path);
        self
    }
    /// Borrows input text deliberately; it may contain secrets.
    pub fn script_input(&self) -> &ScriptInput {
        &self.script
    }
    pub(crate) fn validate(
        mut self,
    ) -> Result<crate::execution::ValidatedShellRequest, CommandPreparationError> {
        if let Err(cause) = self.script.check() {
            return Err(CommandPreparationError { input: self, cause });
        }
        let timing = match std::mem::replace(
            &mut self.timing,
            Err(ValidationError::CorrectionRequired {
                field: "execution timeout",
            }),
        ) {
            Ok(timing) => timing,
            Err(cause) => return Err(CommandPreparationError { input: self, cause }),
        };
        match self.script.validate() {
            Ok(script) => Ok(crate::execution::ValidatedShellRequest {
                script,
                timing,
                cwd: self.cwd,
            }),
            Err(error) => {
                let (script, cause) = error.into_parts();
                Err(CommandPreparationError {
                    input: Self {
                        script,
                        timing: Ok(timing),
                        cwd: self.cwd,
                    },
                    cause,
                })
            }
        }
    }
}
/// Local command validation failed before transmission. Owns the complete draft.
#[derive(Debug, thiserror::Error)]
#[error("command preparation failed: {cause}")]
pub struct CommandPreparationError {
    input: CommandInput,
    #[source]
    cause: ValidationError,
}
impl CommandPreparationError {
    /// Borrows the violation.
    pub fn cause(&self) -> &ValidationError {
        &self.cause
    }
    /// Returns the draft and violation. Replace invalid settings before retrying.
    pub fn into_parts(self) -> (CommandInput, ValidationError) {
        (self.input, self.cause)
    }
}
/// A command borrowing an existing sandbox. Await requires a successful exit.
///
/// This never creates or deletes a sandbox. Default execution and request limit:
/// 30 seconds, bounded by the client's HTTP cap. No automatic retry occurs.
/// Dropping a polled future does not confirm termination of remote work.
#[must_use = "commands do nothing until awaited"]
pub struct Command<'sandbox, 'client> {
    pub(crate) sandbox: &'sandbox Sandbox<'client>,
    pub(crate) input: CommandInput,
}
impl fmt::Debug for Command<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Command([REDACTED])")
    }
}
impl<'sandbox, 'client> Command<'sandbox, 'client> {
    /// Sets the provider and request timeout. The client cap still applies.
    pub fn execution_timeout(mut self, limit: ExecutionTimeout) -> Self {
        self.input = self.input.execution_timeout(limit);
        self
    }
    /// Sets independent execution and HTTP limits. Last timing setter wins.
    pub fn budget(mut self, budget: CommandBudget) -> Self {
        self.input = self.input.budget(budget);
        self
    }
    /// Sets the remote working directory.
    pub fn cwd(mut self, path: SandboxPath) -> Self {
        self.input = self.input.cwd(path);
        self
    }
    #[doc(hidden)]
    pub fn __execution_timeout(mut self, limit: Result<ExecutionTimeout, ValidationError>) -> Self {
        self.input.timing = limit.map(crate::execution::CommandTiming::Coupled);
        self
    }
    /// Executes once and retains any reported exit code, including nonzero or absent.
    pub async fn output(self) -> Result<ShellOutput, ExecutionError> {
        use crate::execution::ShellExecutor;
        let request = self
            .input
            .validate()
            .map_err(|error| ExecutionError::Preparation(Box::new(error)))?;
        self.sandbox.execute(&request).await
    }
    /// Executes once and requires an explicitly reported zero exit status.
    pub async fn execute(self) -> Result<CheckedOutput, ExecutionError> {
        self.output().await?.checked()
    }
}
impl<'sandbox, 'client: 'sandbox> IntoFuture for Command<'sandbox, 'client> {
    type Output = Result<CheckedOutput, ExecutionError>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + 'sandbox>>;
    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.execute())
    }
}
