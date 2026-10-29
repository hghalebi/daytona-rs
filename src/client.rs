use crate::{transport::*, *};
use std::fmt;

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
    pub fn cloud() -> Result<Self, ValidationError> {
        Self::new(
            Endpoint::new("https://app.daytona.io/api/")?,
            Endpoint::new("https://proxy.app.daytona.io/toolbox/")?,
        )
    }
    /// Toolbox is an explicitly trusted credential recipient. Provider-returned
    /// URLs never change this trust boundary; configure self-hosted endpoints here.
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
    pub fn concurrency(mut self, value: ConcurrencyLimit) -> Self {
        self.concurrency = value;
        self
    }
    pub fn response_limit(mut self, value: ResponseLimit) -> Self {
        self.response = value;
        self
    }
    pub fn transfer_limit(mut self, value: TransferLimit) -> Self {
        self.transfer = value;
        self
    }
    pub fn request_timeout(mut self, value: RequestTimeout) -> Self {
        self.request_timeout = value;
        self
    }
    pub fn poll_interval(mut self, value: PollInterval) -> Self {
        self.poll = value;
        self
    }
}
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
    pub fn new(credentials: Credentials) -> Result<Self, ConfigurationError> {
        Self::with_config(credentials, ClientConfig::cloud()?)
    }
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
    pub fn sandboxes(&self) -> Sandboxes<'_> {
        Sandboxes { client: self }
    }
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
pub struct Sandboxes<'client> {
    pub(crate) client: &'client Daytona,
}
impl<'client> Sandboxes<'client> {
    pub fn create(&self) -> Create<Draft> {
        Create::draft()
    }
    pub fn ephemeral(&self) -> Ephemeral<'client> {
        Ephemeral::new(self.client)
    }
}
#[derive(Debug)]
pub struct Sandbox<'client> {
    pub(crate) client: &'client Daytona,
    pub(crate) id: SandboxId,
}
impl Sandbox<'_> {
    pub fn id(&self) -> &SandboxId {
        &self.id
    }
    pub async fn observe(&self) -> Result<Option<SandboxObservation>, OperationError> {
        self.client.observe(&self.id).await
    }
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
pub struct DeletionConfirmed {
    id: SandboxId,
}
impl DeletionConfirmed {
    pub fn sandbox_id(&self) -> &SandboxId {
        &self.id
    }
}
