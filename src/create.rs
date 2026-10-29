use crate::*;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Persist this value before submission for recovery after cancellation. It is
/// correlation data, not proof of acknowledgement or permission to replay.
#[derive(Serialize, Deserialize)]
pub struct CreationReference {
    operation: OperationId,
}
impl CreationReference {
    pub(crate) fn generate() -> Self {
        Self {
            operation: OperationId::generate(),
        }
    }
    pub fn operation(&self) -> &OperationId {
        &self.operation
    }
    pub async fn reconcile(&self, client: &Daytona) -> Result<Reconciliation, OperationError> {
        client.reconcile(self).await
    }
}
impl fmt::Debug for CreationReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CreationReference([REDACTED])")
    }
}
#[derive(Debug)]
pub struct Create<State> {
    state: State,
}
#[derive(Debug)]
pub struct Draft {
    input: RawCreate,
}
#[derive(Debug)]
pub struct Validated {
    pub(crate) specification: CreateSpec,
}
pub struct RawCreate {
    name: Option<SandboxName>,
    source: SandboxSource,
    ttl: Option<SandboxTtl>,
    resources: Option<Resources>,
    network: NetworkPolicy,
    idle: Option<IdleDuration>,
    stopped: Option<StoppedDuration>,
}
pub struct CreateSpec {
    pub(crate) reference: CreationReference,
    pub(crate) name: Option<SandboxName>,
    pub(crate) source: SandboxSource,
    pub(crate) ttl: SandboxTtl,
    pub(crate) resources: Option<Resources>,
    pub(crate) network: NetworkPolicy,
    pub(crate) idle: Option<IdleDuration>,
    pub(crate) stopped: Option<StoppedDuration>,
}
impl fmt::Debug for RawCreate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RawCreate([REDACTED])")
    }
}
impl fmt::Debug for CreateSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CreateSpec([REDACTED])")
    }
}
#[derive(Debug, thiserror::Error)]
#[error("creation input failed validation")]
pub struct CreateValidationError {
    input: Box<Create<Draft>>,
    #[source]
    violation: ValidationError,
}
impl CreateValidationError {
    pub fn into_parts(self) -> (Create<Draft>, ValidationError) {
        (*self.input, self.violation)
    }
    pub fn violation(&self) -> &ValidationError {
        &self.violation
    }
}
impl Create<Draft> {
    pub fn draft() -> Self {
        Self {
            state: Draft {
                input: RawCreate {
                    name: None,
                    source: SandboxSource::Default,
                    ttl: None,
                    resources: None,
                    network: NetworkPolicy::ProviderDefault,
                    idle: None,
                    stopped: None,
                },
            },
        }
    }
    pub fn name(mut self, name: SandboxName) -> Self {
        self.state.input.name = Some(name);
        self
    }
    pub fn source(mut self, source: SandboxSource) -> Self {
        self.state.input.source = source;
        self
    }
    pub fn ttl(mut self, ttl: SandboxTtl) -> Self {
        self.state.input.ttl = Some(ttl);
        self
    }
    pub fn resources(mut self, resources: Resources) -> Self {
        self.state.input.resources = Some(resources);
        self
    }
    pub fn network(mut self, network: NetworkPolicy) -> Self {
        self.state.input.network = network;
        self
    }
    pub fn idle_timeout(mut self, idle: IdleDuration) -> Self {
        self.state.input.idle = Some(idle);
        self
    }
    pub fn delete_after_stop(mut self, stopped: StoppedDuration) -> Self {
        self.state.input.stopped = Some(stopped);
        self
    }
    pub fn input(&self) -> &RawCreate {
        &self.state.input
    }
    pub fn validate(mut self) -> Result<Create<Validated>, CreateValidationError> {
        let ttl = match self.state.input.ttl.take() {
            Some(ttl) => ttl,
            None => {
                return Err(CreateValidationError {
                    input: Box::new(self),
                    violation: ValidationError::MissingTtl,
                });
            }
        };
        let input = self.state.input;
        Ok(Create {
            state: Validated {
                specification: CreateSpec {
                    reference: CreationReference::generate(),
                    name: input.name,
                    source: input.source,
                    ttl,
                    resources: input.resources,
                    network: input.network,
                    idle: input.idle,
                    stopped: input.stopped,
                },
            },
        })
    }
}
impl RawCreate {
    pub fn name(&self) -> Option<&SandboxName> {
        self.name.as_ref()
    }
}
impl Create<Validated> {
    pub(crate) fn with_reference(mut self, reference: CreationReference) -> Self {
        self.state.specification.reference = reference;
        self
    }
    pub fn specification(&self) -> &CreateSpec {
        &self.state.specification
    }
    pub fn reference(&self) -> &CreationReference {
        &self.state.specification.reference
    }
    pub async fn submit(self, client: &Daytona) -> Result<CreationReceipt, CreateError> {
        client.submit(self).await
    }
    pub(crate) fn into_spec(self) -> CreateSpec {
        self.state.specification
    }
}
impl CreateSpec {
    pub fn ttl(&self) -> &SandboxTtl {
        &self.ttl
    }
    pub fn name(&self) -> Option<&SandboxName> {
        self.name.as_ref()
    }
}
#[derive(Debug)]
pub struct RejectedCreate {
    specification: CreateSpec,
}
impl RejectedCreate {
    pub(crate) fn new(specification: CreateSpec) -> Self {
        Self { specification }
    }
    pub fn into_specification(self) -> CreateSpec {
        self.specification
    }
}
#[derive(Debug)]
pub struct UncertainCreation {
    specification: CreateSpec,
}
impl UncertainCreation {
    pub(crate) fn new(specification: CreateSpec) -> Self {
        Self { specification }
    }
    pub fn reference(&self) -> &CreationReference {
        &self.specification.reference
    }
    pub async fn reconcile(&self, client: &Daytona) -> Result<Reconciliation, OperationError> {
        self.reference().reconcile(client).await
    }
}
#[derive(Debug)]
pub enum Reconciliation {
    Found(CreationReceipt),
    /// Empty, ambiguous, or paginated results cannot establish absence/uniqueness.
    Unresolved,
}
#[derive(Debug)]
pub struct CreationReceipt {
    id: SandboxId,
}
impl CreationReceipt {
    pub(crate) fn acknowledged(id: SandboxId) -> Self {
        Self { id }
    }
    pub fn sandbox_id(&self) -> &SandboxId {
        &self.id
    }
    pub fn into_id(self) -> SandboxId {
        self.id
    }
    pub async fn wait(
        self,
        client: &Daytona,
        timeout: WaitTimeout,
    ) -> Result<ReadyObservation, WaitError> {
        client.wait(self, timeout).await
    }
}
#[derive(Debug)]
pub struct ReadyObservation {
    receipt: CreationReceipt,
    observed_at: std::time::SystemTime,
}
impl ReadyObservation {
    pub(crate) fn new(receipt: CreationReceipt) -> Self {
        Self {
            receipt,
            observed_at: std::time::SystemTime::now(),
        }
    }
    pub fn observed_at(&self) -> std::time::SystemTime {
        self.observed_at
    }
    pub fn into_sandbox(self, client: &Daytona) -> Sandbox<'_> {
        client.sandbox(self.receipt.into_id())
    }
}
#[derive(Debug)]
pub enum RemoteState {
    Started,
    Pending,
    Stopped,
    Destroyed,
    Failed,
    Unknown(RemoteStateName),
    Missing,
}
/// Unrecognized provider state, retained without assuming readiness.
#[derive(Debug)]
pub struct RemoteStateName(String);
impl RemoteStateName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl RemoteState {
    pub(crate) fn decode(raw: Option<String>) -> Self {
        match raw {
            None => Self::Missing,
            Some(s) => match s.as_str() {
                "started" => Self::Started,
                "creating" | "starting" | "pending_build" | "building_snapshot"
                | "pulling_snapshot" | "restoring" => Self::Pending,
                "stopped" | "paused" | "archived" => Self::Stopped,
                "destroyed" => Self::Destroyed,
                "error" | "build_failed" => Self::Failed,
                _ => Self::Unknown(RemoteStateName(s)),
            },
        }
    }
}
#[derive(Debug)]
pub struct SandboxObservation {
    pub(crate) state: RemoteState,
    pub(crate) observed_at: std::time::SystemTime,
}
impl SandboxObservation {
    pub fn state(&self) -> &RemoteState {
        &self.state
    }
    pub fn observed_at(&self) -> std::time::SystemTime {
        self.observed_at
    }
}
