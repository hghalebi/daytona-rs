use crate::*;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Persist this value before submission for recovery after cancellation. It is
/// correlation data, not proof of acknowledgement or permission to replay.
#[derive(Serialize, Deserialize)]
/// Persistable correlation data for a creation attempt.
///
/// Serialize it before submitting. Decoding validates its ID but cannot prove
/// acknowledgement. See [recovery](crate::guide::recovery).
pub struct CreationReference {
    operation: OperationId,
}
impl CreationReference {
    pub(crate) fn generate() -> Self {
        Self {
            operation: OperationId::generate(),
        }
    }
    /// Borrows the correlation identity. It is not a provider idempotency key.
    pub fn operation(&self) -> &OperationId {
        &self.operation
    }
    /// Looks up this reference without submitting another create request.
    ///
    /// One unambiguous matching result yields a receipt. Empty, multiple, or
    /// paginated results remain unresolved. See [recovery](crate::guide::recovery).
    ///
    /// # Errors
    /// Returns [`OperationError`] for access, transport, or malformed response failures.
    ///
    /// # Cancellation
    /// Only read-only requests are made; the saved reference remains usable.
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
/// A creation request whose state controls the available local operations.
///
/// Start with [`Create::draft`], validate, then submit. Remote readiness requires
/// a separate observation. See [manual lifecycle](crate::guide::lifecycle).
pub struct Create<State> {
    state: State,
}
#[derive(Debug)]
/// Collected creation input that can still be configured.
///
/// Only [`Create<Draft>`] exposes configuration and validation methods.
pub struct Draft {
    input: RawCreate,
}
#[derive(Debug)]
/// Locally validated creation input ready for one consuming submission.
///
/// It does not prove provider authorization, capacity, or acceptance.
pub struct Validated {
    pub(crate) specification: CreateSpec,
}
/// Collected draft input. Inspect it through [`Create::input`].
pub struct RawCreate {
    name: Option<SandboxName>,
    source: SandboxSource,
    ttl: Option<SandboxTtl>,
    resources: Option<Resources>,
    network: NetworkPolicy,
    idle: Option<IdleDuration>,
    stopped: Option<StoppedDuration>,
}
/// A validated creation specification retained across submission outcomes.
///
/// Debug is redacted. See [`Create::specification`].
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
/// A validation failure retaining the original draft for correction.
///
/// Use [`Self::into_parts`] to recover ownership without cloning.
pub struct CreateValidationError {
    input: Box<Create<Draft>>,
    #[source]
    violation: ValidationError,
}
impl CreateValidationError {
    /// Returns the original draft and violation for correction.
    pub fn into_parts(self) -> (Create<Draft>, ValidationError) {
        (*self.input, self.violation)
    }
    /// Borrows the rule that prevented validation.
    pub fn violation(&self) -> &ValidationError {
        &self.violation
    }
}
impl Create<Draft> {
    /// Creates an empty local draft with provider-default source and networking.
    ///
    /// No network request is made. A positive TTL is required before validation.
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
    /// Sets the requested sandbox name.
    pub fn name(mut self, name: SandboxName) -> Self {
        self.state.input.name = Some(name);
        self
    }
    /// Selects the source, replacing any previous source choice.
    pub fn source(mut self, source: SandboxSource) -> Self {
        self.state.input.source = source;
        self
    }
    /// Sets the required positive provider lifetime in minutes.
    pub fn ttl(mut self, ttl: SandboxTtl) -> Self {
        self.state.input.ttl = Some(ttl);
        self
    }
    /// Sets the requested CPU, memory, and disk allocations.
    pub fn resources(mut self, resources: Resources) -> Self {
        self.state.input.resources = Some(resources);
        self
    }
    /// Sets the outbound network policy requested from the provider.
    pub fn network(mut self, network: NetworkPolicy) -> Self {
        self.state.input.network = network;
        self
    }
    /// Sets the requested idle period before provider auto-stop.
    pub fn idle_timeout(mut self, idle: IdleDuration) -> Self {
        self.state.input.idle = Some(idle);
        self
    }
    /// Sets the requested stopped period before provider auto-delete.
    pub fn delete_after_stop(mut self, stopped: StoppedDuration) -> Self {
        self.state.input.stopped = Some(stopped);
        self
    }
    /// Borrows the collected draft without transferring ownership.
    pub fn input(&self) -> &RawCreate {
        &self.state.input
    }
    /// Consumes the draft and checks locally enforceable creation requirements.
    ///
    /// No API request occurs. Success creates a fresh correlation reference.
    /// See [lifecycle](crate::guide::lifecycle).
    ///
    /// # Errors
    /// Returns [`CreateValidationError`] retaining the draft when its TTL is missing.
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
    /// Borrows the requested name, if configured.
    pub fn name(&self) -> Option<&SandboxName> {
        self.name.as_ref()
    }
}
impl Create<Validated> {
    pub(crate) fn with_reference(mut self, reference: CreationReference) -> Self {
        self.state.specification.reference = reference;
        self
    }
    /// Borrows the validated specification without submitting it.
    pub fn specification(&self) -> &CreateSpec {
        &self.state.specification
    }
    /// Borrows the correlation record. Persist it before awaiting submission.
    pub fn reference(&self) -> &CreationReference {
        &self.state.specification.reference
    }
    /// Consumes the validated request and submits creation once.
    ///
    /// Success means acknowledgement, not readiness. See [lifecycle](crate::guide::lifecycle).
    ///
    /// # Errors
    /// [`CreateError`] distinguishes proven non-submission, rejection, and an unknown
    /// outcome. Retained requests or recovery references preserve useful ownership.
    ///
    /// # Cancellation
    /// Save [`Self::reference`] before awaiting. Cancellation can lose the local
    /// request even when the provider created a sandbox. Reconcile rather than replay.
    pub async fn submit(self, client: &Daytona) -> Result<CreationReceipt, CreateError> {
        client.submit(self).await
    }
    pub(crate) fn into_spec(self) -> CreateSpec {
        self.state.specification
    }
}
impl CreateSpec {
    /// Borrows the finite lifetime requested from the provider.
    pub fn ttl(&self) -> &SandboxTtl {
        &self.ttl
    }
    /// Borrows the requested sandbox name, if present.
    pub fn name(&self) -> Option<&SandboxName> {
        self.name.as_ref()
    }
}
#[derive(Debug)]
/// Intent retained after the provider definitively rejected creation.
pub struct RejectedCreate {
    specification: CreateSpec,
}
impl RejectedCreate {
    pub(crate) fn new(specification: CreateSpec) -> Self {
        Self { specification }
    }
    /// Returns the rejected specification for inspection. It is not a retry handle.
    pub fn into_specification(self) -> CreateSpec {
        self.specification
    }
}
#[derive(Debug)]
/// A creation attempt that may have taken effect.
///
/// Reconcile its reference. This type offers no submission or automatic replay.
/// See [recovery](crate::guide::recovery).
pub struct UncertainCreation {
    specification: CreateSpec,
}
impl UncertainCreation {
    pub(crate) fn new(specification: CreateSpec) -> Self {
        Self { specification }
    }
    /// Borrows the reference to persist or reconcile.
    pub fn reference(&self) -> &CreationReference {
        &self.specification.reference
    }
    /// Queries correlation data without replaying creation.
    ///
    /// # Errors
    /// Returns [`OperationError`] for lookup failures. An unresolved lookup is an
    /// explicit [`Reconciliation::Unresolved`], not proof of absence.
    pub async fn reconcile(&self, client: &Daytona) -> Result<Reconciliation, OperationError> {
        self.reference().reconcile(client).await
    }
}
#[derive(Debug)]
/// The result of looking up a saved creation reference.
pub enum Reconciliation {
    /// One matching sandbox was found; the receipt can be inspected or awaited.
    Found(CreationReceipt),
    /// Empty, ambiguous, or paginated results cannot establish absence/uniqueness.
    Unresolved,
}
#[derive(Debug)]
/// Provider acknowledgement with a known sandbox identity.
///
/// Acknowledgement does not establish readiness. See [`Self::wait`].
pub struct CreationReceipt {
    id: SandboxId,
}
impl CreationReceipt {
    pub(crate) fn acknowledged(id: SandboxId) -> Self {
        Self { id }
    }
    /// Borrows the acknowledged sandbox identity.
    pub fn sandbox_id(&self) -> &SandboxId {
        &self.id
    }
    /// Moves out the identity, for example to construct a handle for cleanup.
    pub fn into_id(self) -> SandboxId {
        self.id
    }
    /// Consumes the receipt and polls until readiness or a terminal outcome.
    ///
    /// The deadline includes observation requests and polling delays.
    /// See [manual lifecycle](crate::guide::lifecycle).
    ///
    /// # Errors
    /// Returns [`WaitError`] retaining the receipt on timeout, terminal state, or
    /// observation failure. The sandbox is not deleted by this method.
    ///
    /// # Cancellation
    /// Persist the ID or creation reference first; dropping this future loses the
    /// owned receipt and does not clean up the remote sandbox.
    pub async fn wait(
        self,
        client: &Daytona,
        timeout: WaitTimeout,
    ) -> Result<ReadyObservation, WaitError> {
        client.wait(self, timeout).await
    }
}
#[derive(Debug)]
/// Evidence that readiness was observed at one point in time.
///
/// It does not lock the sandbox’s future state. See [lifecycle](crate::guide::lifecycle).
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
    /// Returns the local wall-clock time when readiness was recorded.
    pub fn observed_at(&self) -> std::time::SystemTime {
        self.observed_at
    }
    /// Moves the observed identity into a handle borrowing the supplied client.
    ///
    /// This performs no new readiness check. Use the same provider/account context.
    pub fn into_sandbox(self, client: &Daytona) -> Sandbox<'_> {
        client.sandbox(self.receipt.into_id())
    }
}
#[derive(Debug)]
/// A reported state, including unknown and absent provider values.
///
/// This is an observation, not a permanent capability.
pub enum RemoteState {
    /// The provider reported started.
    Started,
    /// The provider reported a creation or startup phase.
    Pending,
    /// The provider reported stopped, paused, or archived.
    Stopped,
    /// The provider reported destroyed.
    Destroyed,
    /// The provider reported an error or build failure.
    Failed,
    /// The provider reported a state this SDK does not recognize.
    Unknown(RemoteStateName),
    /// The response omitted its state.
    Missing,
}
/// Unrecognized provider state, retained without assuming readiness.
#[derive(Debug)]
pub struct RemoteStateName(String);
impl RemoteStateName {
    /// Borrows the provider’s unrecognized state text.
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
/// Remote state together with the local time it was observed.
pub struct SandboxObservation {
    pub(crate) state: RemoteState,
    pub(crate) observed_at: std::time::SystemTime,
}
impl SandboxObservation {
    /// Borrows the reported state; it may already be stale.
    pub fn state(&self) -> &RemoteState {
        &self.state
    }
    /// Returns the local wall-clock time of this observation.
    pub fn observed_at(&self) -> std::time::SystemTime {
        self.observed_at
    }
}
