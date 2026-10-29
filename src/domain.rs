mod input;
pub use input::*;

use serde::{Deserialize, Serialize};
use std::{
    fmt,
    num::{NonZeroU32, NonZeroUsize},
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;

#[derive(Debug, Error)]
/// Input rejected before it becomes a domain value.
///
/// Variants identify the violated rule without retaining the sensitive input.
/// See [validated inputs](crate::guide::inputs).
pub enum ValidationError {
    /// A failed field must be explicitly corrected before retrying local validation.
    #[error("{field} requires correction")]
    CorrectionRequired {
        /// Field that must be replaced with a valid value.
        field: &'static str,
    },
    /// Custom resource sizes require an image source, not a snapshot.
    #[error(
        "custom resources require SandboxSource::Image; snapshots keep their existing resources"
    )]
    ResourcesRequireImage,
    #[error("{field} is empty or contains characters not allowed for this value")]
    /// Text failed the constructor’s format rules.
    InvalidText {
        /// Name of the domain value that failed validation.
        field: &'static str,
    },
    #[error(
        "{field} must be a nonempty ASCII token containing only letters, digits, hyphens or underscores"
    )]
    /// An identity was empty or used a forbidden character.
    InvalidIdentity {
        /// Name of the domain value that failed validation.
        field: &'static str,
    },
    #[error("{field} must be greater than zero")]
    /// A positive quantity was zero.
    Zero {
        /// Name of the domain value that failed validation.
        field: &'static str,
    },
    #[error("{field} exceeds the supported range")]
    /// A quantity exceeded its supported representation.
    Range {
        /// Name of the domain value that failed validation.
        field: &'static str,
    },
    #[error("sandbox paths must be absolute, normalized POSIX paths")]
    /// A remote path was not absolute and normalized.
    InvalidSandboxPath,
    #[error("local path must be nonempty")]
    /// A local path was empty.
    EmptyLocalPath,
    #[error("invalid domain allow list")]
    /// A domain allow list was empty or malformed.
    InvalidDomain,
    #[error("invalid CIDR allow list")]
    /// A CIDR allow list was empty or malformed.
    InvalidCidr,
    #[error("endpoint must be HTTPS (or loopback HTTP), without credentials, query or fragment")]
    /// An endpoint URL violated the transport rules.
    InvalidEndpoint,
    #[error("a finite sandbox TTL is required")]
    /// Creation had no finite positive lifetime.
    MissingTtl,
}

fn text_valid(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}
fn identity_valid(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
/// A validated identifier for a sandbox.
///
/// Contains a nonempty ASCII token: letters, digits, hyphens, or underscores.
/// Deserialization uses the same validation. The value does not prove existence
/// or permission. See the [input lesson](crate::guide::inputs).
pub struct SandboxId(String);
impl SandboxId {
    /// Checks an identifier without contacting Daytona.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidIdentity`] for an empty value or a
    /// character outside ASCII letters, digits, hyphens, and underscores.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !identity_valid(&value) {
            return Err(ValidationError::InvalidIdentity { field: "SandboxId" });
        }
        Ok(Self(value))
    }
    /// Borrows the identifier for serialization or other boundary code.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for SandboxId {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
/// A validated identifier for a session.
///
/// Contains a nonempty ASCII token: letters, digits, hyphens, or underscores.
/// Deserialization uses the same validation. The value does not prove existence
/// or permission. See the [input lesson](crate::guide::inputs).
pub struct SessionId(String);
impl SessionId {
    /// Checks an identifier without contacting Daytona.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidIdentity`] for an empty value or a
    /// character outside ASCII letters, digits, hyphens, and underscores.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !identity_valid(&value) {
            return Err(ValidationError::InvalidIdentity { field: "SessionId" });
        }
        Ok(Self(value))
    }
    /// Borrows the identifier for serialization or other boundary code.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for SessionId {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
/// A validated identifier for a command.
///
/// Contains a nonempty ASCII token: letters, digits, hyphens, or underscores.
/// Deserialization uses the same validation. The value does not prove existence
/// or permission. See the [input lesson](crate::guide::inputs).
pub struct CommandId(String);
impl CommandId {
    /// Checks an identifier without contacting Daytona.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidIdentity`] for an empty value or a
    /// character outside ASCII letters, digits, hyphens, and underscores.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !identity_valid(&value) {
            return Err(ValidationError::InvalidIdentity { field: "CommandId" });
        }
        Ok(Self(value))
    }
    /// Borrows the identifier for serialization or other boundary code.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for CommandId {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
/// A validated identifier for an organization.
///
/// Contains a nonempty ASCII token: letters, digits, hyphens, or underscores.
/// Deserialization uses the same validation. The value does not prove existence
/// or permission. See the [input lesson](crate::guide::inputs).
pub struct OrganizationId(String);
impl OrganizationId {
    /// Checks an identifier without contacting Daytona.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidIdentity`] for an empty value or a
    /// character outside ASCII letters, digits, hyphens, and underscores.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !identity_valid(&value) {
            return Err(ValidationError::InvalidIdentity {
                field: "OrganizationId",
            });
        }
        Ok(Self(value))
    }
    /// Borrows the identifier for serialization or other boundary code.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for OrganizationId {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
/// A validated identifier for a creation operation.
///
/// Contains a nonempty ASCII token: letters, digits, hyphens, or underscores.
/// Deserialization uses the same validation. The value does not prove existence
/// or permission. See the [input lesson](crate::guide::inputs).
pub struct OperationId(String);
impl OperationId {
    /// Checks an identifier without contacting Daytona.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidIdentity`] for an empty value or a
    /// character outside ASCII letters, digits, hyphens, and underscores.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !identity_valid(&value) {
            return Err(ValidationError::InvalidIdentity {
                field: "OperationId",
            });
        }
        Ok(Self(value))
    }
    /// Borrows the identifier for serialization or other boundary code.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for OperationId {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl OperationId {
    pub(crate) fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
/// A display name for a sandbox.
///
/// Must contain non-whitespace text and no control characters. Input is preserved without trimming.
/// Debug output is redacted. Serialization exposes the original text; decoding
/// validates it again. See the [input lesson](crate::guide::inputs).
pub struct SandboxName(String);
impl SandboxName {
    /// Validates text without executing or sending it.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidText`] if the text is blank or contains
    /// a control character.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !(text_valid(&value)) {
            return Err(ValidationError::InvalidText {
                field: "SandboxName",
            });
        }
        Ok(Self(value))
    }
    /// Borrows the original text. It may contain sensitive data; avoid logging it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for SandboxName {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl fmt::Debug for SandboxName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SandboxName([REDACTED])")
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
/// A snapshot name. Snapshot management is outside this SDK’s current API.
///
/// Must contain non-whitespace text and no control characters. Input is preserved without trimming.
/// Debug output is redacted. Serialization exposes the original text; decoding
/// validates it again. See the [input lesson](crate::guide::inputs).
pub struct SnapshotName(String);
impl SnapshotName {
    /// Validates text without executing or sending it.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidText`] if the text is blank or contains
    /// a control character.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !(text_valid(&value)) {
            return Err(ValidationError::InvalidText {
                field: "SnapshotName",
            });
        }
        Ok(Self(value))
    }
    /// Borrows the original text. It may contain sensitive data; avoid logging it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for SnapshotName {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl fmt::Debug for SnapshotName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SnapshotName([REDACTED])")
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
/// A snapshot reference used by [`SandboxSource::Snapshot`].
///
/// Must contain non-whitespace text and no control characters. Input is preserved without trimming.
/// Debug output is redacted. Serialization exposes the original text; decoding
/// validates it again. See the [input lesson](crate::guide::inputs).
pub struct SnapshotRef(String);
impl SnapshotRef {
    /// Validates text without executing or sending it.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidText`] if the text is blank or contains
    /// a control character.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !(text_valid(&value)) {
            return Err(ValidationError::InvalidText {
                field: "SnapshotRef",
            });
        }
        Ok(Self(value))
    }
    /// Borrows the original text. It may contain sensitive data; avoid logging it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for SnapshotRef {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl fmt::Debug for SnapshotRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SnapshotRef([REDACTED])")
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
/// Shell source to execute inside a sandbox.
///
/// Must contain non-whitespace text and no NUL byte. Newlines are allowed. Input is preserved without trimming.
/// Debug output is redacted. Serialization exposes the original text; decoding
/// validates it again. See the [input lesson](crate::guide::inputs).
pub struct ShellScript(String);
impl ShellScript {
    /// Validates text without executing or sending it.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidText`] if the text is blank or contains
    /// a NUL byte.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        Self::validate_text(&value)?;
        Ok(Self(value))
    }
    fn validate_text(value: &str) -> Result<(), ValidationError> {
        if value.trim().is_empty() || value.contains('\0') {
            return Err(ValidationError::InvalidText {
                field: "ShellScript",
            });
        }
        Ok(())
    }
    /// Borrows the original text. It may contain sensitive data; avoid logging it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for ShellScript {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl fmt::Debug for ShellScript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ShellScript([REDACTED])")
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
/// Dockerfile content used by [`SandboxSource::Image`].
///
/// Must contain non-whitespace text and no NUL byte. Newlines are allowed. Input is preserved without trimming.
/// Debug output is redacted. Serialization exposes the original text; decoding
/// validates it again. See the [input lesson](crate::guide::inputs).
pub struct ImageSpec(String);
impl ImageSpec {
    /// Validates text without executing or sending it.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidText`] if the text is blank or contains
    /// a NUL byte.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !(!value.trim().is_empty() && !value.contains('\0')) {
            return Err(ValidationError::InvalidText { field: "ImageSpec" });
        }
        Ok(Self(value))
    }
    /// Borrows the original text. It may contain sensitive data; avoid logging it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for ImageSpec {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl fmt::Debug for ImageSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ImageSpec([REDACTED])")
    }
}

#[derive(PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
/// An API key supplied explicitly by the application.
///
/// Requires nonempty printable ASCII without spaces. Debug output is redacted.
/// Deserialization validates the text but does not authenticate it.
/// See [client setup](crate::guide::configuration).
pub struct ApiKey(String);
impl ApiKey {
    /// Checks the credential text locally.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidText`] for empty input, whitespace,
    /// control characters, or non-ASCII bytes. The provider checks authenticity.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !(!value.is_empty() && value.bytes().all(|c| c.is_ascii_graphic())) {
            return Err(ValidationError::InvalidText { field: "ApiKey" });
        }
        Ok(Self(value))
    }
    /// Exposes the secret as borrowed text. Do not write it to logs or diagnostics.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for ApiKey {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey([REDACTED])")
    }
}

#[derive(PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
/// A JWT token paired with an organization in [`JwtCredentials`].
///
/// Requires nonempty printable ASCII without spaces. Debug output is redacted.
/// Deserialization validates the text but does not authenticate it.
/// See [client setup](crate::guide::configuration).
pub struct JwtToken(String);
impl JwtToken {
    /// Checks the credential text locally.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidText`] for empty input, whitespace,
    /// control characters, or non-ASCII bytes. The provider checks authenticity.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !(!value.is_empty() && value.bytes().all(|c| c.is_ascii_graphic())) {
            return Err(ValidationError::InvalidText { field: "JwtToken" });
        }
        Ok(Self(value))
    }
    /// Exposes the secret as borrowed text. Do not write it to logs or diagnostics.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for JwtToken {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl fmt::Debug for JwtToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JwtToken([REDACTED])")
    }
}

#[derive(PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
/// A preview credential value. Preview discovery is not implemented.
///
/// Requires nonempty printable ASCII without spaces. Debug output is redacted.
/// Deserialization validates the text but does not authenticate it.
/// See [client setup](crate::guide::configuration).
pub struct PreviewCredential(String);
impl PreviewCredential {
    /// Checks the credential text locally.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidText`] for empty input, whitespace,
    /// control characters, or non-ASCII bytes. The provider checks authenticity.
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !(!value.is_empty() && value.bytes().all(|c| c.is_ascii_graphic())) {
            return Err(ValidationError::InvalidText {
                field: "PreviewCredential",
            });
        }
        Ok(Self(value))
    }
    /// Exposes the secret as borrowed text. Do not write it to logs or diagnostics.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for PreviewCredential {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl fmt::Debug for PreviewCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PreviewCredential([REDACTED])")
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32")]
/// Maximum sandbox lifetime requested from the provider.
///
/// Stores 1 through 2,147,483,647 minutes. Serialization uses an integer;
/// deserialization repeats validation. Provider quotas remain runtime checks.
/// See [limits and deadlines](crate::guide::configuration).
///
/// A TTL is a fallback. It does not confirm remote deletion.
pub struct SandboxTtl(NonZeroU32);
impl SandboxTtl {
    /// Creates a positive value in minutes.
    ///
    /// # Errors
    /// Returns [`ValidationError::Zero`] for zero or [`ValidationError::Range`]
    /// above the supported signed 32-bit range.
    pub fn minutes(value: u32) -> Result<Self, ValidationError> {
        // This SDK supports positive signed 32-bit wire integers.
        if value > i32::MAX as u32 {
            return Err(ValidationError::Range {
                field: "SandboxTtl",
            });
        }
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero {
                field: "SandboxTtl",
            })
    }
    /// Returns the validated number of minutes.
    pub fn as_minutes(&self) -> u32 {
        self.0.get()
    }
}
impl TryFrom<u32> for SandboxTtl {
    type Error = ValidationError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::minutes(value)
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32")]
/// Idle time before the provider may stop a sandbox.
///
/// Stores 1 through 2,147,483,647 minutes. Serialization uses an integer;
/// deserialization repeats validation. Provider quotas remain runtime checks.
/// See [limits and deadlines](crate::guide::configuration).
pub struct IdleDuration(NonZeroU32);
impl IdleDuration {
    /// Creates a positive value in minutes.
    ///
    /// # Errors
    /// Returns [`ValidationError::Zero`] for zero or [`ValidationError::Range`]
    /// above the supported signed 32-bit range.
    pub fn minutes(value: u32) -> Result<Self, ValidationError> {
        // This SDK supports positive signed 32-bit wire integers.
        if value > i32::MAX as u32 {
            return Err(ValidationError::Range {
                field: "IdleDuration",
            });
        }
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero {
                field: "IdleDuration",
            })
    }
    /// Returns the validated number of minutes.
    pub fn as_minutes(&self) -> u32 {
        self.0.get()
    }
}
impl TryFrom<u32> for IdleDuration {
    type Error = ValidationError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::minutes(value)
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32")]
/// Time a sandbox may remain stopped before provider deletion.
///
/// Stores 1 through 2,147,483,647 minutes. Serialization uses an integer;
/// deserialization repeats validation. Provider quotas remain runtime checks.
/// See [limits and deadlines](crate::guide::configuration).
pub struct StoppedDuration(NonZeroU32);
impl StoppedDuration {
    /// Creates a positive value in minutes.
    ///
    /// # Errors
    /// Returns [`ValidationError::Zero`] for zero or [`ValidationError::Range`]
    /// above the supported signed 32-bit range.
    pub fn minutes(value: u32) -> Result<Self, ValidationError> {
        // This SDK supports positive signed 32-bit wire integers.
        if value > i32::MAX as u32 {
            return Err(ValidationError::Range {
                field: "StoppedDuration",
            });
        }
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero {
                field: "StoppedDuration",
            })
    }
    /// Returns the validated number of minutes.
    pub fn as_minutes(&self) -> u32 {
        self.0.get()
    }
}
impl TryFrom<u32> for StoppedDuration {
    type Error = ValidationError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::minutes(value)
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32")]
/// Time limit sent for one shell execution.
///
/// Stores 1 through 2,147,483,647 seconds. Serialization uses an integer;
/// deserialization repeats validation. Provider quotas remain runtime checks.
/// See [limits and deadlines](crate::guide::configuration).
///
/// A timeout does not prove that remote work stopped.
pub struct ExecutionTimeout(NonZeroU32);
impl ExecutionTimeout {
    /// Creates a positive value in seconds.
    ///
    /// # Errors
    /// Returns [`ValidationError::Zero`] for zero or [`ValidationError::Range`]
    /// above the supported signed 32-bit range.
    pub fn seconds(value: u32) -> Result<Self, ValidationError> {
        // This SDK supports positive signed 32-bit wire integers.
        if value > i32::MAX as u32 {
            return Err(ValidationError::Range {
                field: "ExecutionTimeout",
            });
        }
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero {
                field: "ExecutionTimeout",
            })
    }
    /// Returns the validated number of seconds.
    pub fn as_seconds(&self) -> u32 {
        self.0.get()
    }
}
impl TryFrom<u32> for ExecutionTimeout {
    type Error = ValidationError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::seconds(value)
    }
}
impl ExecutionTimeout {
    pub(crate) fn duration(&self) -> Duration {
        Duration::from_secs(u64::from(self.0.get()))
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32")]
/// Client HTTP deadline, including reading the response body.
///
/// Stores 1 through 2,147,483,647 seconds. Serialization uses an integer;
/// deserialization repeats validation. Provider quotas remain runtime checks.
/// See [limits and deadlines](crate::guide::configuration).
///
/// A timeout does not prove that remote work stopped.
pub struct RequestTimeout(NonZeroU32);
impl RequestTimeout {
    /// Creates a positive value in seconds.
    ///
    /// # Errors
    /// Returns [`ValidationError::Zero`] for zero or [`ValidationError::Range`]
    /// above the supported signed 32-bit range.
    pub fn seconds(value: u32) -> Result<Self, ValidationError> {
        // This SDK supports positive signed 32-bit wire integers.
        if value > i32::MAX as u32 {
            return Err(ValidationError::Range {
                field: "RequestTimeout",
            });
        }
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero {
                field: "RequestTimeout",
            })
    }
    /// Returns the validated number of seconds.
    pub fn as_seconds(&self) -> u32 {
        self.0.get()
    }
}
impl TryFrom<u32> for RequestTimeout {
    type Error = ValidationError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::seconds(value)
    }
}
impl RequestTimeout {
    pub(crate) fn duration(&self) -> Duration {
        Duration::from_secs(u64::from(self.0.get()))
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32")]
/// Total deadline for a readiness or deletion wait.
///
/// Stores 1 through 2,147,483,647 seconds. Serialization uses an integer;
/// deserialization repeats validation. Provider quotas remain runtime checks.
/// See [limits and deadlines](crate::guide::configuration).
///
/// A timeout does not prove that remote work stopped.
pub struct WaitTimeout(NonZeroU32);
impl WaitTimeout {
    /// Creates a positive value in seconds.
    ///
    /// # Errors
    /// Returns [`ValidationError::Zero`] for zero or [`ValidationError::Range`]
    /// above the supported signed 32-bit range.
    pub fn seconds(value: u32) -> Result<Self, ValidationError> {
        // This SDK supports positive signed 32-bit wire integers.
        if value > i32::MAX as u32 {
            return Err(ValidationError::Range {
                field: "WaitTimeout",
            });
        }
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero {
                field: "WaitTimeout",
            })
    }
    /// Returns the validated number of seconds.
    pub fn as_seconds(&self) -> u32 {
        self.0.get()
    }
}
impl TryFrom<u32> for WaitTimeout {
    type Error = ValidationError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::seconds(value)
    }
}
impl WaitTimeout {
    pub(crate) fn duration(&self) -> Duration {
        Duration::from_secs(u64::from(self.0.get()))
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32")]
/// Delay between read-only state observations.
///
/// Stores 1 through 2,147,483,647 milliseconds. Serialization uses an integer;
/// deserialization repeats validation. Provider quotas remain runtime checks.
/// See [limits and deadlines](crate::guide::configuration).
pub struct PollInterval(NonZeroU32);
impl PollInterval {
    /// Creates a positive value in milliseconds.
    ///
    /// # Errors
    /// Returns [`ValidationError::Zero`] for zero or [`ValidationError::Range`]
    /// above the supported signed 32-bit range.
    pub fn milliseconds(value: u32) -> Result<Self, ValidationError> {
        // This SDK supports positive signed 32-bit wire integers.
        if value > i32::MAX as u32 {
            return Err(ValidationError::Range {
                field: "PollInterval",
            });
        }
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero {
                field: "PollInterval",
            })
    }
    /// Returns the validated number of milliseconds.
    pub fn as_milliseconds(&self) -> u32 {
        self.0.get()
    }
}
impl TryFrom<u32> for PollInterval {
    type Error = ValidationError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::milliseconds(value)
    }
}
impl PollInterval {
    pub(crate) fn duration(&self) -> Duration {
        Duration::from_millis(u64::from(self.0.get()))
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32")]
/// Requested number of CPU cores.
///
/// Stores 1 through 2,147,483,647 cores. Serialization uses an integer;
/// deserialization repeats validation. Provider quotas remain runtime checks.
/// See [limits and deadlines](crate::guide::configuration).
pub struct CpuAllocation(NonZeroU32);
impl CpuAllocation {
    /// Creates a positive value in cores.
    ///
    /// # Errors
    /// Returns [`ValidationError::Zero`] for zero or [`ValidationError::Range`]
    /// above the supported signed 32-bit range.
    pub fn cores(value: u32) -> Result<Self, ValidationError> {
        // This SDK supports positive signed 32-bit wire integers.
        if value > i32::MAX as u32 {
            return Err(ValidationError::Range {
                field: "CpuAllocation",
            });
        }
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero {
                field: "CpuAllocation",
            })
    }
    /// Returns the validated number of cores.
    pub fn as_cores(&self) -> u32 {
        self.0.get()
    }
}
impl TryFrom<u32> for CpuAllocation {
    type Error = ValidationError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::cores(value)
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32")]
/// Requested memory in the provider’s GB units.
///
/// Stores 1 through 2,147,483,647 gigabytes. Serialization uses an integer;
/// deserialization repeats validation. Provider quotas remain runtime checks.
/// See [limits and deadlines](crate::guide::configuration).
pub struct MemorySize(NonZeroU32);
impl MemorySize {
    /// Creates a positive value in gigabytes.
    ///
    /// # Errors
    /// Returns [`ValidationError::Zero`] for zero or [`ValidationError::Range`]
    /// above the supported signed 32-bit range.
    pub fn gigabytes(value: u32) -> Result<Self, ValidationError> {
        // This SDK supports positive signed 32-bit wire integers.
        if value > i32::MAX as u32 {
            return Err(ValidationError::Range {
                field: "MemorySize",
            });
        }
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero {
                field: "MemorySize",
            })
    }
    /// Returns the validated number of gigabytes.
    pub fn as_gigabytes(&self) -> u32 {
        self.0.get()
    }
}
impl TryFrom<u32> for MemorySize {
    type Error = ValidationError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::gigabytes(value)
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32")]
/// Requested disk capacity in the provider’s GB units.
///
/// Stores 1 through 2,147,483,647 gigabytes. Serialization uses an integer;
/// deserialization repeats validation. Provider quotas remain runtime checks.
/// See [limits and deadlines](crate::guide::configuration).
pub struct DiskSize(NonZeroU32);
impl DiskSize {
    /// Creates a positive value in gigabytes.
    ///
    /// # Errors
    /// Returns [`ValidationError::Zero`] for zero or [`ValidationError::Range`]
    /// above the supported signed 32-bit range.
    pub fn gigabytes(value: u32) -> Result<Self, ValidationError> {
        // This SDK supports positive signed 32-bit wire integers.
        if value > i32::MAX as u32 {
            return Err(ValidationError::Range { field: "DiskSize" });
        }
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero { field: "DiskSize" })
    }
    /// Returns the validated number of gigabytes.
    pub fn as_gigabytes(&self) -> u32 {
        self.0.get()
    }
}
impl TryFrom<u32> for DiskSize {
    type Error = ValidationError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::gigabytes(value)
    }
}

#[derive(Debug)]
/// Maximum number of in-flight HTTP operations.
///
/// See [configuration defaults](crate::guide::configuration).
/// There is no waiting queue; excess operations fail before transmission.
pub struct ConcurrencyLimit(NonZeroUsize);
impl ConcurrencyLimit {
    /// Checks a positive limit in operations.
    ///
    /// # Errors
    /// Returns [`ValidationError::Zero`] for zero. Returns [`ValidationError::Range`]
    /// above `u32::MAX` or Tokio’s maximum semaphore permits.
    pub fn new(value: usize) -> Result<Self, ValidationError> {
        if value > u32::MAX as usize || value > tokio::sync::Semaphore::MAX_PERMITS {
            return Err(ValidationError::Range {
                field: "ConcurrencyLimit",
            });
        }
        NonZeroUsize::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero {
                field: "ConcurrencyLimit",
            })
    }
    /// Returns the validated limit in operations.
    pub fn get(&self) -> usize {
        self.0.get()
    }
}

#[derive(Debug)]
/// Maximum captured JSON response size, including shell output.
///
/// See [configuration defaults](crate::guide::configuration).
/// Transfers and responses are buffered in memory within this bound.
pub struct ResponseLimit(NonZeroUsize);
impl ResponseLimit {
    /// Checks a positive limit in bytes.
    ///
    /// # Errors
    /// Returns [`ValidationError::Zero`] for zero. Returns [`ValidationError::Range`]
    /// above `u32::MAX`.
    pub fn new(value: usize) -> Result<Self, ValidationError> {
        if value > u32::MAX as usize {
            return Err(ValidationError::Range {
                field: "ResponseLimit",
            });
        }
        NonZeroUsize::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero {
                field: "ResponseLimit",
            })
    }
    /// Returns the validated limit in bytes.
    pub fn get(&self) -> usize {
        self.0.get()
    }
}

#[derive(Debug)]
/// Maximum captured file size for one upload or download.
///
/// See [configuration defaults](crate::guide::configuration).
/// Transfers and responses are buffered in memory within this bound.
pub struct TransferLimit(NonZeroUsize);
impl TransferLimit {
    /// Checks a positive limit in bytes.
    ///
    /// # Errors
    /// Returns [`ValidationError::Zero`] for zero. Returns [`ValidationError::Range`]
    /// above `u32::MAX`.
    pub fn new(value: usize) -> Result<Self, ValidationError> {
        if value > u32::MAX as usize {
            return Err(ValidationError::Range {
                field: "TransferLimit",
            });
        }
        NonZeroUsize::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero {
                field: "TransferLimit",
            })
    }
    /// Returns the validated limit in bytes.
    pub fn get(&self) -> usize {
        self.0.get()
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16")]
/// A network port from 1 through 65,535.
///
/// Decoding validates the same range. This value does not open a port.
/// See the [input lesson](crate::guide::inputs).
pub struct Port(std::num::NonZeroU16);
impl Port {
    /// Checks a nonzero port number.
    ///
    /// # Errors
    /// Returns [`ValidationError::Zero`] for port zero.
    pub fn new(value: u16) -> Result<Self, ValidationError> {
        std::num::NonZeroU16::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero { field: "Port" })
    }
    /// Returns the port number.
    pub fn get(&self) -> u16 {
        self.0.get()
    }
}
impl TryFrom<u16> for Port {
    type Error = ValidationError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
/// A reported process exit code. Zero means success.
///
/// Every signed 32-bit value is representable. An absent status is represented
/// separately by [`ShellOutput::exit_code`](crate::ShellOutput::exit_code). See [command results](crate::guide::commands).
pub struct ExitCode(i32);
impl ExitCode {
    /// Wraps a reported exit code without changing it.
    pub fn new(value: i32) -> Self {
        Self(value)
    }
    /// Returns whether the reported code is zero.
    pub fn is_success(&self) -> bool {
        self.0 == 0
    }
    /// Returns the reported signed exit code.
    pub fn get(&self) -> i32 {
        self.0
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
/// An absolute, normalized POSIX path inside a sandbox.
///
/// Validation rejects dot segments, duplicate separators, backslashes, NUL bytes,
/// and trailing slashes except for `/`. It does not check remote existence or
/// permissions. Deserialization repeats validation. See [files](crate::guide::files).
pub struct SandboxPath(String);
impl SandboxPath {
    /// Checks a remote path without normalizing or accessing it.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidSandboxPath`] unless the path follows
    /// the rules on [`SandboxPath`].
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !value.starts_with('/')
            || value.contains('\0')
            || value.contains('\\')
            || value.split('/').any(|p| p == ".." || p == ".")
            || value.contains("//")
            || (value.len() > 1 && value.ends_with('/'))
        {
            return Err(ValidationError::InvalidSandboxPath);
        }
        Ok(Self(value))
    }
    /// Borrows the validated POSIX path.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for SandboxPath {
    type Error = ValidationError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug)]
/// A nonempty local path to upload.
///
/// Relative paths are allowed. Validation does not access the filesystem.
/// Existence and permissions are checked during transfer. See [files](crate::guide::files).
pub struct LocalSourcePath(PathBuf);
impl LocalSourcePath {
    /// Checks that the local path is nonempty.
    ///
    /// # Errors
    /// Returns [`ValidationError::EmptyLocalPath`] for an empty path.
    pub fn new(value: impl Into<PathBuf>) -> Result<Self, ValidationError> {
        let value = value.into();
        if value.as_os_str().is_empty() {
            return Err(ValidationError::EmptyLocalPath);
        }
        Ok(Self(value))
    }
    /// Borrows the local filesystem path.
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

#[derive(Debug)]
/// A nonempty local destination for a download.
///
/// Relative paths are allowed. Validation does not access the filesystem.
/// Existence and permissions are checked during transfer. See [files](crate::guide::files).
pub struct LocalDestinationPath(PathBuf);
impl LocalDestinationPath {
    /// Checks that the local path is nonempty.
    ///
    /// # Errors
    /// Returns [`ValidationError::EmptyLocalPath`] for an empty path.
    pub fn new(value: impl Into<PathBuf>) -> Result<Self, ValidationError> {
        let value = value.into();
        if value.as_os_str().is_empty() {
            return Err(ValidationError::EmptyLocalPath);
        }
        Ok(Self(value))
    }
    /// Borrows the local filesystem path.
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<String>")]
/// A nonempty list of allowed ASCII domain names.
///
/// Names may start with `*.`. Each label has 1–63 letters, digits, or hyphens;
/// edge hyphens are rejected. The domain after `*.` has at most 253 bytes.
/// Input case and order are preserved. Decoding validates the same rules.
/// See [`NetworkPolicy`] and [configuration](crate::guide::configuration).
pub struct DomainAllowList(Vec<String>);
impl DomainAllowList {
    /// Checks domain patterns without performing DNS queries.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidDomain`] for an empty list or an invalid
    /// name under the rules on [`DomainAllowList`].
    pub fn new(values: Vec<String>) -> Result<Self, ValidationError> {
        if values.is_empty()
            || values.iter().any(|value| {
                let domain = match value.strip_prefix("*.") {
                    Some(v) => v,
                    None => value,
                };
                domain.len() > 253
                    || domain.split('.').any(|label| {
                        label.is_empty()
                            || label.len() > 63
                            || label.starts_with('-')
                            || label.ends_with('-')
                            || !label
                                .bytes()
                                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
                    })
            })
        {
            return Err(ValidationError::InvalidDomain);
        }
        Ok(Self(values))
    }
    /// Borrows the original domain patterns in input order.
    pub fn as_domains(&self) -> &[String] {
        &self.0
    }
    pub(crate) fn wire(&self) -> String {
        self.0.join(",")
    }
}
impl TryFrom<Vec<String>> for DomainAllowList {
    type Error = ValidationError;
    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<String>")]
/// A nonempty list of IPv4 or IPv6 CIDR network strings.
///
/// Values must parse as `ipnet::IpNet`. Input spelling and order are preserved;
/// decoding repeats validation. See [`NetworkPolicy`].
pub struct CidrAllowList(Vec<String>);
impl CidrAllowList {
    /// Checks every CIDR entry without changing its spelling.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidCidr`] for an empty list or any invalid CIDR.
    pub fn new(values: Vec<String>) -> Result<Self, ValidationError> {
        if values.is_empty() || values.iter().any(|v| v.parse::<ipnet::IpNet>().is_err()) {
            return Err(ValidationError::InvalidCidr);
        }
        Ok(Self(values))
    }
    /// Borrows the validated CIDR strings in input order.
    pub fn as_cidrs(&self) -> &[String] {
        &self.0
    }
    pub(crate) fn wire(&self) -> String {
        self.0.join(",")
    }
}
impl TryFrom<Vec<String>> for CidrAllowList {
    type Error = ValidationError;
    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug)]
/// A validated base URL for the API or toolbox.
///
/// Requires HTTPS, except loopback HTTP. Credentials, queries, and fragments
/// are rejected. A trailing path slash is added when missing. This does not
/// verify server identity beyond transport TLS checks or authenticate a user.
/// See [endpoint configuration](crate::guide::configuration).
pub struct Endpoint(url::Url);
impl Endpoint {
    /// Parses and checks a base URL, adding a trailing path slash if needed.
    ///
    /// # Errors
    /// Returns [`ValidationError::InvalidEndpoint`] for invalid URLs, forbidden
    /// components, missing hosts, or non-loopback plain HTTP.
    pub fn new(value: &str) -> Result<Self, ValidationError> {
        let mut url = url::Url::parse(value).map_err(|_| ValidationError::InvalidEndpoint)?;
        let local = match url.host() {
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            Some(url::Host::Domain("localhost")) => true,
            _ => false,
        };
        if !(url.scheme() == "https" || (url.scheme() == "http" && local))
            || url.host().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(ValidationError::InvalidEndpoint);
        }
        if !url.path().ends_with('/') {
            url.set_path(&format!("{}/", url.path()));
        }
        Ok(Self(url))
    }
    /// Borrows the normalized URL for boundary integration.
    pub fn as_url(&self) -> &url::Url {
        &self.0
    }
    pub(crate) fn join(&self, path: &str) -> Result<url::Url, ValidationError> {
        self.0
            .join(path)
            .map_err(|_| ValidationError::InvalidEndpoint)
    }
}

#[derive(Debug)]
/// A JWT token together with its required organization identity.
///
/// Both values are moved into one credential choice. Debug redacts the token.
/// See [`Credentials`] and [client setup](crate::guide::configuration).
pub struct JwtCredentials {
    token: JwtToken,
    organization: OrganizationId,
}
impl JwtCredentials {
    /// Combines validated values. The provider checks token validity and membership.
    pub fn new(token: JwtToken, organization: OrganizationId) -> Self {
        Self {
            token,
            organization,
        }
    }
    pub(crate) fn parts(&self) -> (&JwtToken, &OrganizationId) {
        (&self.token, &self.organization)
    }
}
#[derive(Debug)]
/// Exactly one authentication method. The client consumes it.
///
/// See [client setup](crate::guide::configuration).
pub enum Credentials {
    /// Authenticate with an API key.
    ApiKey(ApiKey),
    /// Authenticate with a JWT and organization identity.
    Jwt(JwtCredentials),
}
#[derive(Debug)]
/// Exactly one source for a new sandbox. See [`Create::source`](crate::Create::source).
pub enum SandboxSource {
    /// Use the provider’s default sandbox source.
    Default,
    /// Create from an existing snapshot reference.
    Snapshot(SnapshotRef),
    /// Build from Dockerfile content.
    Image(ImageSpec),
}
#[derive(Debug)]
/// Requested outbound network policy. The provider enforces it.
///
/// See [configuration](crate::guide::configuration).
pub enum NetworkPolicy {
    /// Use the provider’s default outbound network behavior.
    ProviderDefault,
    /// Request that all outbound network traffic be blocked.
    BlockAll,
    /// Allow the specified domains and optional CIDR ranges.
    Allow {
        /// Required nonempty domain patterns.
        domains: DomainAllowList,
        /// Additional allowed IP networks, if configured.
        cidrs: Option<CidrAllowList>,
    },
}
#[derive(Debug)]
/// CPU, memory, and disk requirements supplied together.
///
/// Provider capacity and account quotas are checked remotely. See [`Create::resources`](crate::Create::resources).
pub struct Resources {
    pub(crate) cpu: CpuAllocation,
    pub(crate) memory: MemorySize,
    pub(crate) disk: DiskSize,
}
impl Resources {
    /// Moves the three validated allocations into a resource specification.
    pub fn new(cpu: CpuAllocation, memory: MemorySize, disk: DiskSize) -> Self {
        Self { cpu, memory, disk }
    }
}
#[derive(Debug, Serialize, Deserialize)]
/// Serializable sandbox, session, and command identities for application records.
///
/// Deserialization validates each ID. This value is not proof that a command
/// exists or completed. Session/command recovery operations are not implemented.
pub struct CommandReference {
    sandbox: SandboxId,
    session: SessionId,
    command: CommandId,
}
impl CommandReference {
    /// Combines identities without querying the provider.
    pub fn new(sandbox: SandboxId, session: SessionId, command: CommandId) -> Self {
        Self {
            sandbox,
            session,
            command,
        }
    }
    /// Borrows the sandbox identity.
    pub fn sandbox(&self) -> &SandboxId {
        &self.sandbox
    }
    /// Borrows the session identity.
    pub fn session(&self) -> &SessionId {
        &self.session
    }
    /// Borrows the command identity.
    pub fn command(&self) -> &CommandId {
        &self.command
    }
}

/// Independent provider execution and HTTP waiting limits for one command.
///
/// Both inputs are positive validated durations. A shorter HTTP wait is allowed
/// for deliberate early return, but does not stop remote work. The client's hard
/// request cap still applies. See [long commands](crate::guide::long_commands).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandBudget {
    execution: ExecutionTimeout,
    http: RequestTimeout,
}
impl CommandBudget {
    /// Combines validated limits without changing either duration.
    pub fn new(execution: ExecutionTimeout, http: RequestTimeout) -> Self {
        Self { execution, http }
    }
    /// Borrows the provider execution limit.
    pub fn execution(&self) -> &ExecutionTimeout {
        &self.execution
    }
    /// Borrows the per-request HTTP waiting limit.
    pub fn http(&self) -> &RequestTimeout {
        &self.http
    }
}

/// Maximum lifecycle operations waiting for an occupied HTTP permit.
#[derive(Debug)]
pub struct PendingLifecycleLimit(NonZeroUsize);
impl PendingLifecycleLimit {
    /// Checks a positive pending-operation count within Tokio's permit range.
    ///
    /// # Errors
    /// Returns ValidationError for zero or an unsupported semaphore size.
    pub fn new(value: usize) -> Result<Self, ValidationError> {
        if value > tokio::sync::Semaphore::MAX_PERMITS {
            return Err(ValidationError::Range {
                field: "PendingLifecycleLimit",
            });
        }
        NonZeroUsize::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero {
                field: "PendingLifecycleLimit",
            })
    }
    /// Returns the pending-operation count.
    pub fn get(&self) -> usize {
        self.0.get()
    }
}
/// Admission policy for observations and deletion, sharing the ordinary HTTP cap.
#[derive(Debug)]
pub enum LifecycleCapacity {
    /// Reject immediately when the HTTP cap is occupied. This is the default.
    FailFast,
    /// Wait within a bounded queue and deadline. No extra active capacity is added.
    Wait {
        /// Maximum pending lifecycle operations.
        pending: PendingLifecycleLimit,
        /// Maximum capacity wait; outer client and lifecycle deadlines still apply.
        timeout: RequestTimeout,
    },
}
