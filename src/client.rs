use crate::{transport::*, *};
use std::fmt;

/// Endpoints, response bounds, and deadlines for one client.
///
/// Builders consume and return this configuration. They perform no requests.
/// See [defaults and configuration examples](crate::guide::configuration).
pub struct ClientConfig {
    api: Endpoint,
    toolbox: Endpoint,
    concurrency: ConcurrencyLimit,
    response: ResponseLimit,
    transfer: TransferLimit,
    request_timeout: RequestTimeout,
    pub(crate) poll: PollInterval,
}
impl ClientConfig {
    /// Builds configuration for the Daytona cloud API and toolbox.
    ///
    /// Defaults: 16 in-flight requests, 4 MiB JSON, 16 MiB files, a 60-second
    /// HTTP deadline, and 250-millisecond polling.
    ///
    /// # Errors
    /// Returns [`ValidationError`] if a built-in default cannot be validated.
    pub fn cloud() -> Result<Self, ValidationError> {
        Self::new(
            Endpoint::new("https://app.daytona.io/api/")?,
            Endpoint::new("https://proxy.app.daytona.io/toolbox/")?,
        )
    }
    /// Toolbox is an explicitly trusted credential recipient. Provider-returned
    /// URLs never change this trust boundary; configure self-hosted endpoints here.
    /// Builds configuration for explicit API and toolbox endpoints.
    ///
    /// The toolbox receives credentials, so select a trusted endpoint. Other defaults
    /// match [`Self::cloud`]. See [configuration](crate::guide::configuration).
    ///
    /// # Errors
    /// Returns [`ValidationError`] if a built-in resource limit cannot be validated.
    pub fn new(api: Endpoint, toolbox: Endpoint) -> Result<Self, ValidationError> {
        Ok(Self {
            api,
            toolbox,
            concurrency: ConcurrencyLimit::new(16)?,
            response: ResponseLimit::new(4_194_304)?,
            transfer: TransferLimit::new(16_777_216)?,
            request_timeout: RequestTimeout::seconds(60)?,
            poll: PollInterval::milliseconds(250)?,
        })
    }
    /// Sets the in-flight HTTP limit. Default: 16.
    ///
    /// There is no waiting queue. Excess requests return [`TransportError::Busy`].
    pub fn concurrency(mut self, value: ConcurrencyLimit) -> Self {
        self.concurrency = value;
        self
    }
    /// Sets the JSON response and shell-output capture limit in bytes. Default: 4 MiB.
    pub fn response_limit(mut self, value: ResponseLimit) -> Self {
        self.response = value;
        self
    }
    /// Sets the in-memory file transfer limit in bytes. Default: 16 MiB.
    pub fn transfer_limit(mut self, value: TransferLimit) -> Self {
        self.transfer = value;
        self
    }
    /// Sets the HTTP deadline, including the response body. Default: 60 seconds.
    ///
    /// Expiration does not prove that remote work stopped.
    pub fn request_timeout(mut self, value: RequestTimeout) -> Self {
        self.request_timeout = value;
        self
    }
    /// Sets the delay between state observations. Default: 250 milliseconds.
    pub fn poll_interval(mut self, value: PollInterval) -> Self {
        self.poll = value;
        self
    }
}
/// Owns the HTTP connection pool and authentication credentials.
///
/// Create one client, then borrow it for sandbox operations. Handles cannot
/// outlive this client. No environment variables are read implicitly.
/// See [getting started](crate::guide::getting_started).
pub struct Daytona {
    pub(crate) transport: Transport,
    api: Endpoint,
    toolbox: Endpoint,
    pub(crate) transfer_limit: TransferLimit,
    pub(crate) poll: PollInterval,
}
impl fmt::Debug for Daytona {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Daytona([REDACTED])")
    }
}
impl Daytona {
    /// Creates a client using cloud endpoints and default limits.
    ///
    /// Consumes credentials. No authentication request is made yet. Use the client
    /// within a Tokio runtime. See [getting started](crate::guide::getting_started).
    ///
    /// # Errors
    /// Returns [`ConfigurationError`] for invalid defaults, header encoding, or
    /// HTTP transport initialization failure.
    pub fn new(credentials: Credentials) -> Result<Self, ConfigurationError> {
        Self::with_config(credentials, ClientConfig::cloud()?)
    }
    /// Creates a client with explicit credentials and configuration.
    ///
    /// No API request is sent. See [configuration](crate::guide::configuration).
    ///
    /// # Errors
    /// Returns [`ConfigurationError`] if credential headers or the transport cannot
    /// be constructed.
    pub fn with_config(
        credentials: Credentials,
        config: ClientConfig,
    ) -> Result<Self, ConfigurationError> {
        Ok(Self {
            transport: Transport::new(
                credentials,
                config.concurrency,
                config.response,
                config.request_timeout,
            )?,
            api: config.api,
            toolbox: config.toolbox,
            transfer_limit: config.transfer,
            poll: config.poll,
        })
    }
    /// Borrows this client to prepare a create request or temporary scope.
    pub fn sandboxes(&self) -> Sandboxes<'_> {
        Sandboxes { client: self }
    }
    /// Moves a known ID into a handle borrowing this client.
    ///
    /// This performs no lookup. Call [`Sandbox::observe`] to inspect current state.
    pub fn sandbox(&self, id: SandboxId) -> Sandbox<'_> {
        Sandbox { client: self, id }
    }
    pub(crate) fn api_url(&self, suffix: &str) -> Result<url::Url, OperationError> {
        self.api
            .join(suffix)
            .map_err(|e| ProtocolError::from(e).into())
    }
    pub(crate) fn toolbox_url(
        &self,
        id: &SandboxId,
        suffix: &str,
    ) -> Result<url::Url, OperationError> {
        self.toolbox
            .join(&format!("{}/{suffix}", id.as_str()))
            .map_err(|e| ProtocolError::from(e).into())
    }
    pub(crate) async fn submit(
        &self,
        request: Create<Validated>,
    ) -> Result<CreationReceipt, CreateError> {
        let url = match self.api_url("sandbox") {
            Ok(url) => url,
            Err(cause) => {
                return Err(CreateError::NotSubmitted {
                    request: Box::new(request),
                    cause,
                });
            }
        };
        let dto = CreateDto::from(request.specification());
        let outgoing = self
            .transport
            .request(reqwest::Method::POST, url)
            .json(&dto);
        let response = match self.transport.send(outgoing).await {
            Ok(response) => response,
            Err(DeliveryFailure::Before(cause)) => {
                return Err(CreateError::NotSubmitted {
                    request: Box::new(request),
                    cause: cause.into(),
                });
            }
            Err(DeliveryFailure::After(cause)) => {
                return Err(CreateError::OutcomeUnknown {
                    recovery: Box::new(UncertainCreation::new(request.into_spec())),
                    cause: cause.into(),
                });
            }
        };
        // 408 may be emitted after forwarding; ambiguous gateway/server responses
        // are never considered safe to retry.
        if matches!(
            response.status,
            400 | 401 | 403 | 404 | 405 | 409 | 413 | 415 | 422 | 429
        ) {
            return Err(CreateError::Rejected {
                intent: Box::new(RejectedCreate::new(request.into_spec())),
                cause: ProviderRejection::new(response.status),
            });
        }
        match response.decode::<SandboxDto>() {
            Ok(dto) => Ok(CreationReceipt::acknowledged(dto.id)),
            Err(cause) => Err(CreateError::OutcomeUnknown {
                recovery: Box::new(UncertainCreation::new(request.into_spec())),
                cause,
            }),
        }
    }
    pub(crate) async fn reconcile(
        &self,
        reference: &CreationReference,
    ) -> Result<Reconciliation, OperationError> {
        let mut url = self.api_url("sandbox")?;
        let labels = serde_json::to_string(&std::collections::BTreeMap::from([(
            CORRELATION_LABEL,
            reference.operation().as_str(),
        )]))
        .map_err(|_| ProtocolError::Malformed)?;
        url.query_pairs_mut()
            .append_pair("labels", &labels)
            .append_pair("limit", "2");
        let dto: ListDto = self
            .transport
            .send(self.transport.request(reqwest::Method::GET, url))
            .await
            .map_err(DeliveryFailure::into_operation)?
            .decode()?;
        if dto.next_cursor.is_some() || dto.items.len() != 1 {
            return Ok(Reconciliation::Unresolved);
        }
        match dto.items.into_iter().next() {
            Some(item)
                if item
                    .labels
                    .get(CORRELATION_LABEL)
                    .is_some_and(|value| value == reference.operation().as_str()) =>
            {
                Ok(Reconciliation::Found(CreationReceipt::acknowledged(
                    item.id,
                )))
            }
            _ => Ok(Reconciliation::Unresolved),
        }
    }
    pub(crate) async fn wait(
        &self,
        receipt: CreationReceipt,
        timeout: WaitTimeout,
    ) -> Result<ReadyObservation, WaitError> {
        match poll_ready(
            self,
            &TokioClock,
            receipt.sandbox_id(),
            &timeout,
            &self.poll,
        )
        .await
        {
            Ok(Readiness::Ready) => Ok(ReadyObservation::new(receipt)),
            Ok(Readiness::Deadline) => Err(WaitError::Deadline { receipt }),
            Ok(Readiness::Terminal) => Err(WaitError::Terminal { receipt }),
            Err(cause) => Err(WaitError::Observation { receipt, cause }),
        }
    }
}
impl SandboxControl for Daytona {
    async fn observe(&self, id: &SandboxId) -> Result<Option<SandboxObservation>, OperationError> {
        let url = self.api_url(&format!("sandbox/{}", id.as_str()))?;
        let response = self
            .transport
            .send(self.transport.request(reqwest::Method::GET, url))
            .await
            .map_err(DeliveryFailure::into_operation)?;
        if response.status == 404 {
            return Ok(None);
        }
        let dto: SandboxDto = response.decode()?;
        if &dto.id != id {
            return Err(ProtocolError::IdentityMismatch.into());
        }
        Ok(Some(SandboxObservation {
            state: RemoteState::decode(dto.state),
            observed_at: std::time::SystemTime::now(),
        }))
    }
}
/// A borrowed entry point for creating sandbox requests and scopes.
///
/// Obtained from [`Daytona::sandboxes`]. See [getting started](crate::guide::getting_started).
pub struct Sandboxes<'client> {
    pub(crate) client: &'client Daytona,
}
impl<'client> Sandboxes<'client> {
    /// Starts a local draft. Set a TTL, validate it, then submit it.
    ///
    /// See [manual lifecycle](crate::guide::lifecycle).
    pub fn create(&self) -> Create<Draft> {
        Create::draft()
    }
    /// Starts a scope that owns its sandbox and handles normal-completion cleanup.
    ///
    /// Set a TTL before running work. See [getting started](crate::guide::getting_started).
    pub fn ephemeral(&self) -> Ephemeral<'client> {
        Ephemeral::new(self.client)
    }
}
#[derive(Debug)]
/// An owned sandbox identity with a borrowed client.
///
/// The handle does not guarantee current remote state or access. Operations
/// check those facts at runtime. Dropping a handle does not delete its sandbox.
/// See [ownership](crate::guide::ownership) and [manual cleanup](crate::guide::lifecycle).
pub struct Sandbox<'client> {
    pub(crate) client: &'client Daytona,
    pub(crate) id: SandboxId,
}
impl Sandbox<'_> {
    /// Borrows the identity carried by this handle.
    pub fn id(&self) -> &SandboxId {
        &self.id
    }
    /// Reads the current remote state without changing it.
    ///
    /// Returns `None` for HTTP 404. A returned observation may become stale.
    /// Unknown or omitted states remain explicit.
    ///
    /// # Errors
    /// Returns [`OperationError`] for transport, protocol, or access failures.
    pub async fn observe(&self) -> Result<Option<SandboxObservation>, OperationError> {
        self.client.observe(&self.id).await
    }
    /// Consumes the handle, submits deletion, and waits for confirmed removal.
    ///
    /// Success requires HTTP 404 or an observed destroyed state. The total deadline
    /// covers submission and polling. See [manual lifecycle](crate::guide::lifecycle).
    ///
    /// # Errors
    /// Returns [`CleanupError`] with the sandbox ID and deletion acceptance when
    /// removal cannot be confirmed.
    ///
    /// # Cancellation
    /// Dropping this future can lose the local handle while remote deletion is
    /// unresolved. Persist its identity before awaiting when recovery is required.
    pub async fn delete(self, timeout: WaitTimeout) -> Result<DeletionConfirmed, CleanupError> {
        let Sandbox { client, id } = self;
        let url = match client.api_url(&format!("sandbox/{}", id.as_str())) {
            Ok(url) => url,
            Err(cause) => {
                return Err(CleanupError::Unconfirmed {
                    sandbox: id,
                    acceptance: DeleteAcceptance::Unknown,
                    cause,
                });
            }
        };
        let end = match tokio::time::Instant::now().checked_add(timeout.duration()) {
            Some(end) => end,
            None => {
                return Err(CleanupError::Deadline {
                    sandbox: id,
                    acceptance: DeleteAcceptance::Unknown,
                });
            }
        };
        let delete = match tokio::time::timeout_at(
            end,
            client
                .transport
                .send_headers(client.transport.request(reqwest::Method::DELETE, url)),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => {
                return Err(CleanupError::Deadline {
                    sandbox: id,
                    acceptance: DeleteAcceptance::Unknown,
                });
            }
        };
        let acceptance = match delete {
            Ok(response) if response.status == 404 => return Ok(DeletionConfirmed { id }),
            Ok(response) if (200..300).contains(&response.status) => DeleteAcceptance::Acknowledged,
            Ok(response) => {
                return Err(CleanupError::Unconfirmed {
                    sandbox: id,
                    acceptance: DeleteAcceptance::Unknown,
                    cause: ProtocolError::Status {
                        status: response.status,
                    }
                    .into(),
                });
            }
            Err(e) => {
                return Err(CleanupError::Unconfirmed {
                    sandbox: id,
                    acceptance: DeleteAcceptance::Unknown,
                    cause: e.into_operation(),
                });
            }
        };
        loop {
            match tokio::time::timeout_at(end, client.observe(&id)).await {
                Ok(Ok(None)) => return Ok(DeletionConfirmed { id }),
                Ok(Ok(Some(observation)))
                    if matches!(observation.state, RemoteState::Destroyed) =>
                {
                    return Ok(DeletionConfirmed { id });
                }
                Ok(Err(cause)) => {
                    return Err(CleanupError::Unconfirmed {
                        sandbox: id,
                        acceptance,
                        cause,
                    });
                }
                Err(_) => {
                    return Err(CleanupError::Deadline {
                        sandbox: id,
                        acceptance,
                    });
                }
                Ok(Ok(Some(_))) => {}
            }
            if tokio::time::timeout_at(end, TokioClock.sleep(&client.poll))
                .await
                .is_err()
                || tokio::time::Instant::now() >= end
            {
                return Err(CleanupError::Deadline {
                    sandbox: id,
                    acceptance,
                });
            }
        }
    }
}
#[derive(Debug)]
/// Evidence that absence or the destroyed state was observed.
///
/// A successful DELETE response alone is not enough. Returned by [`Sandbox::delete`].
pub struct DeletionConfirmed {
    id: SandboxId,
}
impl DeletionConfirmed {
    /// Borrows the identity whose removal was confirmed.
    pub fn sandbox_id(&self) -> &SandboxId {
        &self.id
    }
}
