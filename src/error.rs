use crate::{
    Create, CreationReceipt, RejectedCreate, SandboxId, ShellOutput, UncertainCreation, Validated,
    ValidationError,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error("could not build the HTTP transport")]
    Transport,
    #[error("credentials cannot be encoded as HTTP headers")]
    Credentials,
}

/// Sanitized transport classification. Raw URLs, bodies and credentials never
/// enter diagnostics. A timeout is not evidence that a remote effect stopped.
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("the configured concurrency limit is occupied; request was not sent")]
    Busy,
    #[error("request could not be constructed; request was not sent")]
    Build,
    #[error("request deadline elapsed; remote outcome may be unknown")]
    Timeout,
    #[error("connection failed; remote outcome may be unknown")]
    Connection,
    #[error("response exceeded the configured byte limit")]
    BodyLimit,
}
impl TransportError {
    pub(crate) fn from_http(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::Timeout
        } else {
            Self::Connection
        }
    }
}
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("response did not satisfy the protocol schema")]
    Malformed,
    #[error("response identity did not match the requested identity")]
    IdentityMismatch,
    #[error("provider returned HTTP {status}")]
    Status { status: u16 },
    #[error(transparent)]
    Validation(#[from] ValidationError),
}
#[derive(Debug, Error)]
#[error("authentication was rejected")]
pub struct AuthenticationError;
#[derive(Debug, Error)]
#[error("operation was not authorized")]
pub struct AuthorizationError;
#[derive(Debug, Error)]
pub enum ProviderRejection {
    #[error(transparent)]
    Authentication(#[from] AuthenticationError),
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
    #[error("provider rejected the request with HTTP {status}")]
    Status { status: u16 },
}
impl ProviderRejection {
    pub(crate) fn new(status: u16) -> Self {
        match status {
            401 => Self::Authentication(AuthenticationError),
            403 => Self::Authorization(AuthorizationError),
            status => Self::Status { status },
        }
    }
}
#[derive(Debug, Error)]
pub enum OperationError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error(transparent)]
    Rejected(#[from] ProviderRejection),
}
#[derive(Debug, Error)]
pub enum CreateError {
    #[error("creation failed before submission")]
    NotSubmitted {
        request: Box<Create<Validated>>,
        #[source]
        cause: OperationError,
    },
    #[error("the provider rejected creation")]
    Rejected {
        intent: Box<RejectedCreate>,
        #[source]
        cause: ProviderRejection,
    },
    #[error("creation outcome is uncertain; reconcile before taking further action")]
    OutcomeUnknown {
        recovery: Box<UncertainCreation>,
        #[source]
        cause: OperationError,
    },
}
#[derive(Debug, Error)]
pub enum WaitError {
    #[error("readiness deadline elapsed; receipt retained")]
    Deadline { receipt: CreationReceipt },
    #[error("readiness observation failed; receipt retained")]
    Observation {
        receipt: CreationReceipt,
        #[source]
        cause: OperationError,
    },
    #[error("sandbox reached a terminal state before readiness; receipt retained")]
    Terminal { receipt: CreationReceipt },
}
impl WaitError {
    pub fn into_receipt(self) -> CreationReceipt {
        match self {
            Self::Deadline { receipt }
            | Self::Observation { receipt, .. }
            | Self::Terminal { receipt } => receipt,
        }
    }
}
#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error(transparent)]
    Operation(#[from] OperationError),
    #[error("command response has no exit status")]
    MissingExitStatus { output: ShellOutput },
    #[error("command exited unsuccessfully")]
    NonZeroExit { output: ShellOutput },
}
#[derive(Debug, Error)]
pub enum TransferError {
    #[error(transparent)]
    Operation(#[from] OperationError),
    #[error("file contents exceed the configured transfer limit")]
    Limit,
    #[error("local file operation failed")]
    Io(#[from] std::io::Error),
    #[error("destination may contain a partial file after a local write failure")]
    PartialWrite(#[source] std::io::Error),
}
#[derive(Debug, Error)]
pub enum CleanupError {
    #[error("deletion could not be confirmed; resource identity retained")]
    Unconfirmed {
        sandbox: SandboxId,
        acceptance: DeleteAcceptance,
        #[source]
        cause: OperationError,
    },
    #[error("deletion confirmation deadline elapsed; resource identity retained")]
    Deadline {
        sandbox: SandboxId,
        acceptance: DeleteAcceptance,
    },
}
#[derive(Debug)]
pub enum DeleteAcceptance {
    Acknowledged,
    Unknown,
}
impl CleanupError {
    pub fn sandbox_id(&self) -> &SandboxId {
        match self {
            Self::Unconfirmed { sandbox, .. } | Self::Deadline { sandbox, .. } => sandbox,
        }
    }
    pub fn acceptance(&self) -> &DeleteAcceptance {
        match self {
            Self::Unconfirmed { acceptance, .. } | Self::Deadline { acceptance, .. } => acceptance,
        }
    }
}
