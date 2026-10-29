use crate::{transport::*, *};
use std::fmt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Captured command text and an optional reported exit code.
///
/// Owns its data and may outlive the sandbox. Debug redacts text.
/// See [command results](crate::guide::commands).
pub struct ShellOutput {
    text: String,
    exit: Option<ExitCode>,
}
impl fmt::Debug for ShellOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShellOutput")
            .field("text", &"[REDACTED]")
            .field("exit", &self.exit)
            .finish()
    }
}
impl ShellOutput {
    /// Borrows captured text. It may contain secrets; display it deliberately.
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Borrows the reported exit code, or returns `None` when it was omitted.
    pub fn exit_code(&self) -> Option<&ExitCode> {
        self.exit.as_ref()
    }
    /// Consumes this output and requires an explicit zero exit code.
    ///
    /// # Errors
    /// Returns [`ExecutionError::NonZeroExit`] for a nonzero code, or
    /// [`ExecutionError::MissingExitStatus`] for an absent code. Both retain output.
    pub fn checked(self) -> Result<CheckedOutput, ExecutionError> {
        match &self.exit {
            Some(exit) if exit.is_success() => Ok(CheckedOutput(self)),
            Some(_) => Err(ExecutionError::NonZeroExit { output: self }),
            None => Err(ExecutionError::MissingExitStatus { output: self }),
        }
    }
}
#[derive(Debug)]
/// Owned command output with an explicitly reported zero exit code.
///
/// Constructed only by checking [`ShellOutput`]. See [first command](crate::guide::getting_started).
pub struct CheckedOutput(ShellOutput);
impl CheckedOutput {
    /// Borrows the captured text from the successful command.
    pub fn text(&self) -> &str {
        self.0.text()
    }
    /// Moves out the original output, including its reported exit code.
    pub fn into_output(self) -> ShellOutput {
        self.0
    }
}
pub(crate) struct ValidatedShellRequest {
    pub(crate) script: ShellScript,
    pub(crate) timing: CommandTiming,
    pub(crate) cwd: Option<SandboxPath>,
}
#[derive(Debug)]
pub(crate) enum CommandTiming {
    Coupled(ExecutionTimeout),
    Separate(CommandBudget),
}
/// No retries: failure after transmission may leave a running command.
pub(crate) trait ShellExecutor {
    async fn execute(&self, request: &ValidatedShellRequest)
    -> Result<ShellOutput, ExecutionError>;
}
impl<'client> Sandbox<'client> {
    /// Prepares a checked command in this sandbox. No creation or deletion occurs.
    ///
    /// Strings validate before transmission. Await checks the exit status; use
    /// Command::output to inspect arbitrary exits. Default execution limit: 30 seconds.
    pub fn run(&self, script: impl Into<ScriptInput>) -> Command<'_, 'client> {
        self.command(CommandInput::new(script))
    }
    /// Resumes a corrected command draft without copying its input.
    pub fn command(&self, input: CommandInput) -> Command<'_, 'client> {
        Command {
            sandbox: self,
            input,
        }
    }
    /// Downloads a bounded remote file into a newly created local file.
    ///
    /// An existing destination is never overwritten. Content is buffered in memory.
    /// See [download lesson](crate::guide::files::download).
    ///
    /// # Errors
    /// Returns [`TransferError`] for remote, limit, or local I/O failures.
    /// [`TransferError::PartialWrite`] means the new destination may be incomplete.
    ///
    /// # Cancellation
    /// Cancellation can leave a partial local file. Inspect it before reusing its path.
    pub async fn download(
        &self,
        remote: &SandboxPath,
        local: &LocalDestinationPath,
    ) -> Result<TransferReceipt, TransferError> {
        let contents = self.read_file(remote).await?;
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(local.as_path())
            .await?;
        file.write_all(&contents.0)
            .await
            .map_err(TransferError::PartialWrite)?;
        file.flush().await.map_err(TransferError::PartialWrite)?;
        Ok(TransferReceipt {
            bytes: TransferredBytes(contents.0.len()),
        })
    }
    /// Uploads a bounded local file to the explicit remote path.
    ///
    /// The remote destination may be overwritten. Content is buffered in memory.
    /// See [upload lesson](crate::guide::files::upload).
    ///
    /// # Errors
    /// Returns [`TransferError`] for local reads, size limits, transport, protocol,
    /// or permission failures. An uncertain response may follow a successful write.
    ///
    /// # Cancellation
    /// Cancellation does not undo a remote write. Read back the file when its result matters.
    pub async fn upload(
        &self,
        local: &LocalSourcePath,
        remote: &SandboxPath,
    ) -> Result<TransferReceipt, TransferError> {
        let file = tokio::fs::File::open(local.as_path()).await?;
        let limit = self.client.transfer_limit.get();
        let read_limit = u64::try_from(limit)
            .map_err(|_| TransferError::Limit)?
            .checked_add(1)
            .ok_or(TransferError::Limit)?;
        let mut contents = Vec::new();
        file.take(read_limit).read_to_end(&mut contents).await?;
        if contents.len() > limit {
            return Err(TransferError::Limit);
        }
        self.write_file(remote, FileContents(contents)).await
    }
}
impl ShellExecutor for Sandbox<'_> {
    async fn execute(
        &self,
        request: &ValidatedShellRequest,
    ) -> Result<ShellOutput, ExecutionError> {
        let url = self.client.toolbox_url(&self.id, "process/execute")?;
        let timeout = match &request.timing {
            CommandTiming::Coupled(t) => t.as_seconds(),
            CommandTiming::Separate(budget) => budget.execution().as_seconds(),
        };
        let dto = ExecuteDto {
            command: request.script.as_str(),
            timeout,
            cwd: request.cwd.as_ref().map(SandboxPath::as_str),
        };
        let outgoing = self
            .client
            .transport
            .request(reqwest::Method::POST, url)
            .json(&dto);
        let outgoing = match &request.timing {
            CommandTiming::Coupled(t) => outgoing.timeout(t.duration()),
            CommandTiming::Separate(budget) => outgoing.timeout(budget.http().duration()),
        };
        let response: OutputDto = self
            .client
            .transport
            .send(outgoing)
            .await
            .map_err(DeliveryFailure::into_operation)?
            .decode()?;
        Ok(ShellOutput {
            text: response.result,
            exit: response.exit_code,
        })
    }
}
pub(crate) struct FileContents(Vec<u8>);
pub(crate) trait FileTransfer {
    async fn read_file(&self, path: &SandboxPath) -> Result<FileContents, TransferError>;
    async fn write_file(
        &self,
        path: &SandboxPath,
        contents: FileContents,
    ) -> Result<TransferReceipt, TransferError>;
}
impl FileTransfer for Sandbox<'_> {
    async fn read_file(&self, path: &SandboxPath) -> Result<FileContents, TransferError> {
        let mut url = self.client.toolbox_url(&self.id, "files/download")?;
        url.query_pairs_mut().append_pair("path", path.as_str());
        let response = self
            .client
            .transport
            .send_bounded(
                self.client.transport.request(reqwest::Method::GET, url),
                self.client.transfer_limit.get(),
            )
            .await
            .map_err(|failure| match failure {
                DeliveryFailure::After(TransportError::BodyLimit) => TransferError::Limit,
                other => TransferError::Operation(other.into_operation()),
            })?
            .success()?;
        Ok(FileContents(response.body))
    }
    async fn write_file(
        &self,
        path: &SandboxPath,
        contents: FileContents,
    ) -> Result<TransferReceipt, TransferError> {
        let bytes = TransferredBytes(contents.0.len());
        let mut url = self.client.toolbox_url(&self.id, "files/upload-v2")?;
        url.query_pairs_mut().append_pair("path", path.as_str());
        self.client
            .transport
            .send(
                self.client
                    .transport
                    .request(reqwest::Method::POST, url)
                    .header("Content-Type", "application/octet-stream")
                    .body(contents.0),
            )
            .await
            .map_err(DeliveryFailure::into_operation)?
            .success()?;
        Ok(TransferReceipt { bytes })
    }
}
#[derive(Debug)]
/// The number of bytes in a completed file transfer.
pub struct TransferredBytes(usize);
impl TransferredBytes {
    /// Returns the transferred byte count.
    pub fn get(&self) -> usize {
        self.0
    }
}
#[derive(Debug)]
/// A completed transfer and its byte count.
///
/// An upload receipt acknowledges provider acceptance; it is not a checksum
/// verification or a durability guarantee. See [files](crate::guide::files).
pub struct TransferReceipt {
    bytes: TransferredBytes,
}
impl TransferReceipt {
    /// Borrows the byte count for this completed transfer.
    pub fn bytes(&self) -> &TransferredBytes {
        &self.bytes
    }
}
