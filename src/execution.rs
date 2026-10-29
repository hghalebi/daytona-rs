use crate::{transport::*, *};
use std::fmt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn exit_code(&self) -> Option<&ExitCode> {
        self.exit.as_ref()
    }
    pub fn checked(self) -> Result<CheckedOutput, ExecutionError> {
        match &self.exit {
            Some(exit) if exit.is_success() => Ok(CheckedOutput(self)),
            Some(_) => Err(ExecutionError::NonZeroExit { output: self }),
            None => Err(ExecutionError::MissingExitStatus { output: self }),
        }
    }
}
#[derive(Debug)]
pub struct CheckedOutput(ShellOutput);
impl CheckedOutput {
    pub fn text(&self) -> &str {
        self.0.text()
    }
    pub fn into_output(self) -> ShellOutput {
        self.0
    }
}
pub struct Shell<'sandbox, 'client> {
    sandbox: &'sandbox Sandbox<'client>,
    request: ValidatedShellRequest,
}
pub(crate) struct ValidatedShellRequest {
    script: ShellScript,
    timeout: Option<ExecutionTimeout>,
    cwd: Option<SandboxPath>,
}
/// No retries: failure after transmission may leave a running command.
pub(crate) trait ShellExecutor {
    async fn execute(&self, request: &ValidatedShellRequest)
    -> Result<ShellOutput, ExecutionError>;
}
impl<'client> Sandbox<'client> {
    pub fn shell(&self, script: ShellScript) -> Shell<'_, 'client> {
        Shell {
            sandbox: self,
            request: ValidatedShellRequest {
                script,
                timeout: None,
                cwd: None,
            },
        }
    }
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
    /// Upload overwrites the specified remote file; a timeout may leave it changed.
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
impl Shell<'_, '_> {
    pub fn execution_timeout(mut self, timeout: ExecutionTimeout) -> Self {
        self.request.timeout = Some(timeout);
        self
    }
    pub fn cwd(mut self, path: SandboxPath) -> Self {
        self.request.cwd = Some(path);
        self
    }
    pub async fn output(self) -> Result<ShellOutput, ExecutionError> {
        self.sandbox.execute(&self.request).await
    }
    pub async fn checked_output(self) -> Result<CheckedOutput, ExecutionError> {
        self.output().await?.checked()
    }
}
impl ShellExecutor for Sandbox<'_> {
    async fn execute(
        &self,
        request: &ValidatedShellRequest,
    ) -> Result<ShellOutput, ExecutionError> {
        let url = self.client.toolbox_url(&self.id, "process/execute")?;
        let timeout = match &request.timeout {
            Some(t) => t.as_seconds(),
            None => 10,
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
        let outgoing = match &request.timeout {
            Some(t) => outgoing.timeout(t.duration()),
            None => outgoing,
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
            .map_err(DeliveryFailure::into_operation)?
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
pub struct TransferredBytes(usize);
impl TransferredBytes {
    pub fn get(&self) -> usize {
        self.0
    }
}
#[derive(Debug)]
pub struct TransferReceipt {
    bytes: TransferredBytes,
}
impl TransferReceipt {
    pub fn bytes(&self) -> &TransferredBytes {
        &self.bytes
    }
}
