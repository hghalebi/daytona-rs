use crate::{
    Create, CreationReceipt, RejectedCreate, SandboxId, ShellOutput, UncertainCreation, Validated,
    ValidationError,
};
use thiserror::Error;

#[derive(Debug, Error)]
/// A client could not be configured locally. No API request was sent.
pub enum ConfigurationError {
    #[error(transparent)]
    /// A local value failed validation.
    Validation(#[from] ValidationError),
    #[error("could not build the HTTP transport")]
    /// The HTTP transport could not be initialized.
    Transport,
    #[error("credentials cannot be encoded as HTTP headers")]
    /// Credentials could not be encoded as HTTP headers.
    Credentials,
}

#[derive(Debug, Error)]
/// A bounded HTTP operation could not produce a usable response.
///
/// Interpret transmission evidence in its operation context; a timeout does not
/// make a mutation safe to replay. See [recovery](crate::guide::recovery).
pub enum TransportError {
    /// Capacity waiting expired. This request was not sent.
    #[error("capacity deadline elapsed; request was not sent")]
    CapacityDeadline,
    #[error("the configured concurrency limit is occupied; request was not sent")]
    /// The concurrency limit was full. This request was not sent.
    Busy,
    #[error("request could not be constructed; request was not sent")]
    /// The request could not be built. This request was not sent.
    Build,
    #[error("request deadline elapsed; remote outcome may be unknown")]
    /// The deadline elapsed. The remote outcome may be unknown.
    Timeout,
    #[error("connection failed; remote outcome may be unknown")]
    /// The connection failed. Transmission or remote completion may be uncertain.
    Connection,
    #[error("response exceeded the configured byte limit")]
    /// The response body exceeded its configured capture limit.
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
/// A response could not establish the required protocol facts.
pub enum ProtocolError {
    #[error("response did not satisfy the protocol schema")]
    /// The response did not match the required schema.
    Malformed,
    #[error("response identity did not match the requested identity")]
    /// The response identified a different sandbox.
    IdentityMismatch,
    #[error("provider returned HTTP {status}")]
    /// The provider returned an unexpected HTTP status.
    Status {
        /// Sanitized HTTP status code.
        status: u16,
    },
    #[error(transparent)]
    /// A local value failed validation.
    Validation(#[from] ValidationError),
}
#[derive(Debug, Error)]
#[error("authentication was rejected")]
/// The provider rejected authentication. Check the credential without logging it.
pub struct AuthenticationError;
#[derive(Debug, Error)]
#[error("operation was not authorized")]
/// The provider denied this operation. Check account and organization permissions.
pub struct AuthorizationError;
#[derive(Debug, Error)]
/// A definitive provider rejection with sanitized status context.
pub enum ProviderRejection {
    #[error(transparent)]
    /// The provider rejected the supplied authentication.
    Authentication(#[from] AuthenticationError),
    #[error(transparent)]
    /// The provider denied permission.
    Authorization(#[from] AuthorizationError),
    #[error("provider rejected the request with HTTP {status}")]
    /// The provider returned an unexpected HTTP status.
    Status {
        /// Sanitized HTTP status code.
        status: u16,
    },
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
/// A transport, protocol, or provider failure for a borrowed operation.
///
/// The caller still owns the target context. See [troubleshooting](crate::guide::troubleshooting).
pub enum OperationError {
    #[error(transparent)]
    /// The HTTP operation failed; interpret uncertainty in the caller’s context.
    Transport(#[from] TransportError),
    #[error(transparent)]
    /// The response did not establish the required facts.
    Protocol(#[from] ProtocolError),
    #[error(transparent)]
    /// The provider definitively rejected the operation.
    Rejected(#[from] ProviderRejection),
}
#[derive(Debug, Error)]
/// A consuming creation failure that preserves useful request or recovery data.
///
/// Only `NotSubmitted` establishes that this attempt was not transmitted.
/// See [recovery](crate::guide::recovery).
pub enum CreateError {
    #[error("creation failed before submission")]
    /// Creation was proven not transmitted. The validated request is retained.
    NotSubmitted {
        /// Owned validated request that was not submitted.
        request: Box<Create<Validated>>,
        #[source]
        /// Underlying sanitized failure.
        cause: OperationError,
    },
    #[error("the provider rejected creation")]
    /// The provider definitively rejected the operation.
    Rejected {
        /// Original rejected intent for inspection.
        intent: Box<RejectedCreate>,
        #[source]
        /// Underlying sanitized failure.
        cause: ProviderRejection,
    },
    #[error("creation outcome is uncertain; reconcile before taking further action")]
    /// Creation may have happened. Reconcile the retained reference before further action.
    OutcomeUnknown {
        /// Owned reconciliation data; it cannot be replayed as a request.
        recovery: Box<UncertainCreation>,
        #[source]
        /// Underlying sanitized failure.
        cause: OperationError,
    },
}
#[derive(Debug, Error)]
/// Readiness was not established; the owned receipt is retained.
///
/// Recover it with [`Self::into_receipt`] for inspection or cleanup.
pub enum WaitError {
    #[error("readiness deadline elapsed; receipt retained")]
    /// The observation deadline elapsed; the remote outcome is not established.
    Deadline {
        /// Acknowledged identity retained for recovery or cleanup.
        receipt: CreationReceipt,
    },
    #[error("readiness observation failed; receipt retained")]
    /// A readiness observation failed; the receipt is retained.
    Observation {
        /// Acknowledged identity retained for recovery or cleanup.
        receipt: CreationReceipt,
        #[source]
        /// Underlying sanitized failure.
        cause: OperationError,
    },
    #[error("sandbox reached a terminal state before readiness; receipt retained")]
    /// A terminal state was observed before readiness.
    Terminal {
        /// Acknowledged identity retained for recovery or cleanup.
        receipt: CreationReceipt,
    },
}
impl WaitError {
    /// Moves out the receipt for inspection or cleanup after failed readiness.
    pub fn into_receipt(self) -> CreationReceipt {
        match self {
            Self::Deadline { receipt }
            | Self::Observation { receipt, .. }
            | Self::Terminal { receipt } => receipt,
        }
    }
}
#[derive(Debug, Error)]
/// Execution failed or a successful exit could not be established.
///
/// Exit-related variants retain captured output. See [commands](crate::guide::commands).
pub enum ExecutionError {
    #[error(transparent)]
    /// The remote operation failed at the transport, protocol, or access boundary.
    Operation(#[from] OperationError),
    #[error("command response has no exit status")]
    /// The response omitted an exit code. Success cannot be assumed.
    MissingExitStatus {
        /// Captured command output, including its optional exit status.
        output: ShellOutput,
    },
    #[error("command exited unsuccessfully")]
    /// The command reported a nonzero exit code.
    NonZeroExit {
        /// Captured command output, including its optional exit status.
        output: ShellOutput,
    },
}
impl ExecutionError {
    /// Borrows output retained by a nonzero or missing exit status.
    ///
    /// Returns None for operation failures. Text is exposed only through
    /// ShellOutput::text; Debug remains redacted.
    pub fn output(&self) -> Option<&ShellOutput> {
        match self {
            Self::NonZeroExit { output } | Self::MissingExitStatus { output } => Some(output),
            Self::Operation(_) => None,
        }
    }
}

/// Owned evidence from an unconfirmed deletion.
#[derive(Debug)]
pub struct CleanupParts {
    /// Identity for an explicit recovery action.
    pub sandbox: SandboxId,
    /// Known acceptance of this deletion request.
    pub acceptance: DeleteAcceptance,
    /// Why confirmation ended, including its cause when available.
    pub reason: CleanupReason,
}
/// Why deletion confirmation ended.
#[derive(Debug, Error)]
pub enum CleanupReason {
    /// The overall confirmation deadline elapsed.
    #[error("deletion confirmation deadline elapsed")]
    Deadline,
    /// An operation failed before confirmation.
    #[error(transparent)]
    Operation(#[from] OperationError),
}

#[derive(Debug, Error)]
/// A bounded upload or download failed. See [files](crate::guide::files).
pub enum TransferError {
    #[error(transparent)]
    /// The remote operation failed at the transport, protocol, or access boundary.
    Operation(#[from] OperationError),
    #[error("file contents exceed the configured transfer limit")]
    /// The file exceeded the configured transfer limit.
    Limit,
    #[error("local file operation failed")]
    /// A local filesystem operation failed.
    Io(#[from] std::io::Error),
    #[error("destination may contain a partial file after a local write failure")]
    /// A local write failed. The newly created destination may be incomplete.
    PartialWrite(#[source] std::io::Error),
}
#[derive(Debug, Error)]
/// Deletion could not be confirmed. The sandbox ID remains available.
///
/// Inspect acceptance separately from confirmation. See [errors](crate::guide::errors).
pub enum CleanupError {
    #[error("deletion could not be confirmed; resource identity retained")]
    /// Deletion could not be confirmed after an operation failure.
    Unconfirmed {
        /// Owned identity to use for explicit cleanup recovery.
        sandbox: SandboxId,
        /// Whether deletion was acknowledged before confirmation failed.
        acceptance: DeleteAcceptance,
        #[source]
        /// Underlying sanitized failure.
        cause: OperationError,
    },
    #[error("deletion confirmation deadline elapsed; resource identity retained")]
    /// The observation deadline elapsed; the remote outcome is not established.
    Deadline {
        /// Owned identity to use for explicit cleanup recovery.
        sandbox: SandboxId,
        /// Whether deletion was acknowledged before confirmation failed.
        acceptance: DeleteAcceptance,
    },
}
#[derive(Debug)]
/// What is known about the deletion request, independent of confirmed removal.
pub enum DeleteAcceptance {
    /// This deletion attempt was proven not transmitted. Earlier attempts may exist.
    NotSubmitted,
    /// The provider rejected this attempt. This does not describe earlier attempts.
    Rejected,
    /// The provider accepted deletion, but removal has not been confirmed.
    Acknowledged,
    /// Deletion acceptance is unknown.
    Unknown,
}
impl CleanupError {
    /// Consumes the failure without discarding identity, acceptance, or cause.
    pub fn into_parts(self) -> CleanupParts {
        match self {
            Self::Unconfirmed {
                sandbox,
                acceptance,
                cause,
            } => CleanupParts {
                sandbox,
                acceptance,
                reason: CleanupReason::Operation(cause),
            },
            Self::Deadline {
                sandbox,
                acceptance,
            } => CleanupParts {
                sandbox,
                acceptance,
                reason: CleanupReason::Deadline,
            },
        }
    }

    /// Borrows the identity whose deletion remains unconfirmed.
    pub fn sandbox_id(&self) -> &SandboxId {
        match self {
            Self::Unconfirmed { sandbox, .. } | Self::Deadline { sandbox, .. } => sandbox,
        }
    }
    /// Borrows the known deletion-acceptance state.
    pub fn acceptance(&self) -> &DeleteAcceptance {
        match self {
            Self::Unconfirmed { acceptance, .. } | Self::Deadline { acceptance, .. } => acceptance,
        }
    }
}
