use serde::{Deserialize, Serialize};
use std::{
    fmt,
    num::{NonZeroU32, NonZeroUsize},
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("{field} must be nonempty and contain no control characters")]
    InvalidText { field: &'static str },
    #[error(
        "{field} must be a nonempty ASCII token containing only letters, digits, hyphens or underscores"
    )]
    InvalidIdentity { field: &'static str },
    #[error("{field} must be greater than zero")]
    Zero { field: &'static str },
    #[error("{field} exceeds the supported range")]
    Range { field: &'static str },
    #[error("sandbox paths must be absolute, normalized POSIX paths")]
    InvalidSandboxPath,
    #[error("local path must be nonempty")]
    EmptyLocalPath,
    #[error("invalid domain allow list")]
    InvalidDomain,
    #[error("invalid CIDR allow list")]
    InvalidCidr,
    #[error("endpoint must be HTTPS (or loopback HTTP), without credentials, query or fragment")]
    InvalidEndpoint,
    #[error("a finite sandbox TTL is required")]
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
pub struct SandboxId(String);
impl SandboxId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !identity_valid(&value) {
            return Err(ValidationError::InvalidIdentity { field: "SandboxId" });
        }
        Ok(Self(value))
    }
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
pub struct SessionId(String);
impl SessionId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !identity_valid(&value) {
            return Err(ValidationError::InvalidIdentity { field: "SessionId" });
        }
        Ok(Self(value))
    }
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
pub struct CommandId(String);
impl CommandId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !identity_valid(&value) {
            return Err(ValidationError::InvalidIdentity { field: "CommandId" });
        }
        Ok(Self(value))
    }
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
pub struct OrganizationId(String);
impl OrganizationId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !identity_valid(&value) {
            return Err(ValidationError::InvalidIdentity {
                field: "OrganizationId",
            });
        }
        Ok(Self(value))
    }
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
pub struct OperationId(String);
impl OperationId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !identity_valid(&value) {
            return Err(ValidationError::InvalidIdentity {
                field: "OperationId",
            });
        }
        Ok(Self(value))
    }
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
pub struct SandboxName(String);
impl SandboxName {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !(text_valid(&value)) {
            return Err(ValidationError::InvalidText {
                field: "SandboxName",
            });
        }
        Ok(Self(value))
    }
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
pub struct SnapshotName(String);
impl SnapshotName {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !(text_valid(&value)) {
            return Err(ValidationError::InvalidText {
                field: "SnapshotName",
            });
        }
        Ok(Self(value))
    }
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
pub struct SnapshotRef(String);
impl SnapshotRef {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !(text_valid(&value)) {
            return Err(ValidationError::InvalidText {
                field: "SnapshotRef",
            });
        }
        Ok(Self(value))
    }
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
pub struct ShellScript(String);
impl ShellScript {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !(!value.trim().is_empty() && !value.contains('\0')) {
            return Err(ValidationError::InvalidText {
                field: "ShellScript",
            });
        }
        Ok(Self(value))
    }
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
pub struct ImageSpec(String);
impl ImageSpec {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !(!value.trim().is_empty() && !value.contains('\0')) {
            return Err(ValidationError::InvalidText { field: "ImageSpec" });
        }
        Ok(Self(value))
    }
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
pub struct ApiKey(String);
impl ApiKey {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !(!value.is_empty() && value.bytes().all(|c| c.is_ascii_graphic())) {
            return Err(ValidationError::InvalidText { field: "ApiKey" });
        }
        Ok(Self(value))
    }
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
pub struct JwtToken(String);
impl JwtToken {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !(!value.is_empty() && value.bytes().all(|c| c.is_ascii_graphic())) {
            return Err(ValidationError::InvalidText { field: "JwtToken" });
        }
        Ok(Self(value))
    }
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
pub struct PreviewCredential(String);
impl PreviewCredential {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if !(!value.is_empty() && value.bytes().all(|c| c.is_ascii_graphic())) {
            return Err(ValidationError::InvalidText {
                field: "PreviewCredential",
            });
        }
        Ok(Self(value))
    }
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
pub struct SandboxTtl(NonZeroU32);
impl SandboxTtl {
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
pub struct IdleDuration(NonZeroU32);
impl IdleDuration {
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
pub struct StoppedDuration(NonZeroU32);
impl StoppedDuration {
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
pub struct ExecutionTimeout(NonZeroU32);
impl ExecutionTimeout {
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
pub struct RequestTimeout(NonZeroU32);
impl RequestTimeout {
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
pub struct WaitTimeout(NonZeroU32);
impl WaitTimeout {
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
pub struct PollInterval(NonZeroU32);
impl PollInterval {
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
pub struct CpuAllocation(NonZeroU32);
impl CpuAllocation {
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
pub struct MemorySize(NonZeroU32);
impl MemorySize {
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
pub struct DiskSize(NonZeroU32);
impl DiskSize {
    pub fn gigabytes(value: u32) -> Result<Self, ValidationError> {
        // This SDK supports positive signed 32-bit wire integers.
        if value > i32::MAX as u32 {
            return Err(ValidationError::Range { field: "DiskSize" });
        }
        NonZeroU32::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero { field: "DiskSize" })
    }
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
pub struct ConcurrencyLimit(NonZeroUsize);
impl ConcurrencyLimit {
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
    pub fn get(&self) -> usize {
        self.0.get()
    }
}

#[derive(Debug)]
pub struct ResponseLimit(NonZeroUsize);
impl ResponseLimit {
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
    pub fn get(&self) -> usize {
        self.0.get()
    }
}

#[derive(Debug)]
pub struct TransferLimit(NonZeroUsize);
impl TransferLimit {
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
    pub fn get(&self) -> usize {
        self.0.get()
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16")]
pub struct Port(std::num::NonZeroU16);
impl Port {
    pub fn new(value: u16) -> Result<Self, ValidationError> {
        std::num::NonZeroU16::new(value)
            .map(Self)
            .ok_or(ValidationError::Zero { field: "Port" })
    }
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
pub struct ExitCode(i32);
impl ExitCode {
    pub fn new(value: i32) -> Self {
        Self(value)
    }
    pub fn is_success(&self) -> bool {
        self.0 == 0
    }
    pub fn get(&self) -> i32 {
        self.0
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct SandboxPath(String);
impl SandboxPath {
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
pub struct LocalSourcePath(PathBuf);
impl LocalSourcePath {
    pub fn new(value: impl Into<PathBuf>) -> Result<Self, ValidationError> {
        let value = value.into();
        if value.as_os_str().is_empty() {
            return Err(ValidationError::EmptyLocalPath);
        }
        Ok(Self(value))
    }
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

#[derive(Debug)]
pub struct LocalDestinationPath(PathBuf);
impl LocalDestinationPath {
    pub fn new(value: impl Into<PathBuf>) -> Result<Self, ValidationError> {
        let value = value.into();
        if value.as_os_str().is_empty() {
            return Err(ValidationError::EmptyLocalPath);
        }
        Ok(Self(value))
    }
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<String>")]
pub struct DomainAllowList(Vec<String>);
impl DomainAllowList {
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
pub struct CidrAllowList(Vec<String>);
impl CidrAllowList {
    pub fn new(values: Vec<String>) -> Result<Self, ValidationError> {
        if values.is_empty() || values.iter().any(|v| v.parse::<ipnet::IpNet>().is_err()) {
            return Err(ValidationError::InvalidCidr);
        }
        Ok(Self(values))
    }
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
pub struct Endpoint(url::Url);
impl Endpoint {
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
pub struct JwtCredentials {
    token: JwtToken,
    organization: OrganizationId,
}
impl JwtCredentials {
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
pub enum Credentials {
    ApiKey(ApiKey),
    Jwt(JwtCredentials),
}
#[derive(Debug)]
pub enum SandboxSource {
    Default,
    Snapshot(SnapshotRef),
    Image(ImageSpec),
}
#[derive(Debug)]
pub enum NetworkPolicy {
    ProviderDefault,
    BlockAll,
    Allow {
        domains: DomainAllowList,
        cidrs: Option<CidrAllowList>,
    },
}
#[derive(Debug)]
pub struct Resources {
    pub(crate) cpu: CpuAllocation,
    pub(crate) memory: MemorySize,
    pub(crate) disk: DiskSize,
}
impl Resources {
    pub fn new(cpu: CpuAllocation, memory: MemorySize, disk: DiskSize) -> Self {
        Self { cpu, memory, disk }
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CommandReference {
    sandbox: SandboxId,
    session: SessionId,
    command: CommandId,
}
impl CommandReference {
    pub fn new(sandbox: SandboxId, session: SessionId, command: CommandId) -> Self {
        Self {
            sandbox,
            session,
            command,
        }
    }
    pub fn sandbox(&self) -> &SandboxId {
        &self.sandbox
    }
    pub fn session(&self) -> &SessionId {
        &self.session
    }
    pub fn command(&self) -> &CommandId {
        &self.command
    }
}
