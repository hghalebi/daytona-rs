//! Raw protocol representations and the bounded network boundary.
use crate::*;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

pub(crate) struct Transport {
    http: reqwest::Client,
    credentials: Credentials,
    permits: Semaphore,
    limit: ResponseLimit,
    timeout: RequestTimeout,
}
pub(crate) enum DeliveryFailure {
    Before(TransportError),
    After(TransportError),
}
enum Capture {
    HeadersOnly,
    Bounded(usize),
}
pub(crate) struct WireResponse {
    pub status: u16,
    pub body: Vec<u8>,
}
impl Transport {
    pub fn new(
        credentials: Credentials,
        concurrency: ConcurrencyLimit,
        limit: ResponseLimit,
        timeout: RequestTimeout,
    ) -> Result<Self, ConfigurationError> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(timeout.duration())
            .timeout(timeout.duration())
            .build()
            .map_err(|_| ConfigurationError::Transport)?;
        Ok(Self {
            http,
            credentials,
            permits: Semaphore::new(concurrency.get()),
            limit,
            timeout,
        })
    }
    pub fn request(&self, method: reqwest::Method, url: url::Url) -> reqwest::RequestBuilder {
        let request = self.http.request(method, url);
        match &self.credentials {
            Credentials::ApiKey(key) => request.bearer_auth(key.as_str()),
            Credentials::Jwt(jwt) => {
                let (token, organization) = jwt.parts();
                request
                    .bearer_auth(token.as_str())
                    .header("X-Daytona-Organization-ID", organization.as_str())
            }
        }
    }
    pub async fn send(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<WireResponse, DeliveryFailure> {
        self.send_bounded(request, self.limit.get()).await
    }
    pub async fn send_bounded(
        &self,
        request: reqwest::RequestBuilder,
        limit: usize,
    ) -> Result<WireResponse, DeliveryFailure> {
        self.send_with_capture(request, Capture::Bounded(limit))
            .await
    }
    pub async fn send_headers(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<WireResponse, DeliveryFailure> {
        self.send_with_capture(request, Capture::HeadersOnly).await
    }
    async fn send_with_capture(
        &self,
        request: reqwest::RequestBuilder,
        capture: Capture,
    ) -> Result<WireResponse, DeliveryFailure> {
        let request = request
            .build()
            .map_err(|_| DeliveryFailure::Before(TransportError::Build))?;
        let _permit = self
            .permits
            .try_acquire()
            .map_err(|_| DeliveryFailure::Before(TransportError::Busy))?;
        // Covers body consumption as well as connection establishment.
        let operation = async {
            let mut response = self
                .http
                .execute(request)
                .await
                .map_err(TransportError::from_http)?;
            let status = response.status().as_u16();
            if !(200..300).contains(&status) {
                return Ok(WireResponse {
                    status,
                    body: Vec::new(),
                });
            }
            let limit = match capture {
                Capture::HeadersOnly => {
                    return Ok(WireResponse {
                        status,
                        body: Vec::new(),
                    });
                }
                Capture::Bounded(limit) => limit,
            };
            if response.content_length().is_some_and(|n| n > limit as u64) {
                return Err(TransportError::BodyLimit);
            }
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(TransportError::from_http)? {
                let size = body
                    .len()
                    .checked_add(chunk.len())
                    .ok_or(TransportError::BodyLimit)?;
                if size > limit {
                    return Err(TransportError::BodyLimit);
                }
                body.extend_from_slice(&chunk);
            }
            Ok(WireResponse { status, body })
        };
        match tokio::time::timeout(self.timeout.duration(), operation).await {
            Ok(result) => result.map_err(DeliveryFailure::After),
            Err(_) => Err(DeliveryFailure::After(TransportError::Timeout)),
        }
    }
}
impl DeliveryFailure {
    pub fn into_operation(self) -> OperationError {
        match self {
            Self::Before(e) | Self::After(e) => e.into(),
        }
    }
}
impl WireResponse {
    pub fn success(self) -> Result<Self, OperationError> {
        match self.status {
            200..=299 => Ok(self),
            400..=499 => Err(ProviderRejection::new(self.status).into()),
            status => Err(ProtocolError::Status { status }.into()),
        }
    }
    pub fn decode<T: serde::de::DeserializeOwned>(self) -> Result<T, OperationError> {
        let response = self.success()?;
        serde_json::from_slice(&response.body).map_err(|_| ProtocolError::Malformed.into())
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateDto<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a SandboxName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<&'a SnapshotRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    build_info: Option<BuildDto<'a>>,
    ttl_minutes: u32,
    auto_delete_interval: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_stop_interval: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disk: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    network_block_all: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    domain_allow_list: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    network_allow_list: Option<String>,
    labels: std::collections::BTreeMap<&'static str, &'a str>,
    public: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildDto<'a> {
    dockerfile_content: &'a str,
}
pub(crate) const CORRELATION_LABEL: &str = "daytona-rs.operation";
impl<'a> From<&'a CreateSpec> for CreateDto<'a> {
    fn from(spec: &'a CreateSpec) -> Self {
        let (snapshot, build_info) = match &spec.source {
            SandboxSource::Default => (None, None),
            SandboxSource::Snapshot(snapshot) => (Some(snapshot), None),
            SandboxSource::Image(image) => (
                None,
                Some(BuildDto {
                    dockerfile_content: image.as_str(),
                }),
            ),
        };
        let (network_block_all, domain_allow_list, network_allow_list) = match &spec.network {
            NetworkPolicy::ProviderDefault => (None, None, None),
            NetworkPolicy::BlockAll => (Some(true), None, None),
            NetworkPolicy::Allow { domains, cidrs } => (
                Some(false),
                Some(domains.wire()),
                cidrs.as_ref().map(CidrAllowList::wire),
            ),
        };
        Self {
            name: spec.name.as_ref(),
            snapshot,
            build_info,
            ttl_minutes: spec.ttl.as_minutes(),
            auto_delete_interval: match &spec.stopped {
                Some(s) => s.as_minutes(),
                None => 0,
            },
            auto_stop_interval: spec.idle.as_ref().map(IdleDuration::as_minutes),
            cpu: spec.resources.as_ref().map(|r| r.cpu.as_cores()),
            memory: spec.resources.as_ref().map(|r| r.memory.as_gigabytes()),
            disk: spec.resources.as_ref().map(|r| r.disk.as_gigabytes()),
            network_block_all,
            domain_allow_list,
            network_allow_list,
            labels: std::collections::BTreeMap::from([(
                CORRELATION_LABEL,
                spec.reference.operation().as_str(),
            )]),
            public: false,
        }
    }
}
#[derive(Deserialize)]
pub(crate) struct SandboxDto {
    pub id: SandboxId,
    pub state: Option<String>,
    #[serde(default)]
    pub labels: std::collections::BTreeMap<String, String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListDto {
    pub items: Vec<SandboxDto>,
    #[serde(deserialize_with = "required_cursor")]
    pub next_cursor: Option<String>,
}
fn required_cursor<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(decoder)
}
#[derive(Serialize)]
pub(crate) struct ExecuteDto<'a> {
    pub command: &'a str,
    pub timeout: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<&'a str>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OutputDto {
    pub result: String,
    pub exit_code: Option<ExitCode>,
}

/// Observation is read-only; implementations must preserve unknown state and
/// return errors rather than manufacture readiness.
pub(crate) trait SandboxControl {
    async fn observe(&self, id: &SandboxId) -> Result<Option<SandboxObservation>, OperationError>;
}
pub(crate) trait Clock {
    fn now(&self) -> tokio::time::Instant;
    async fn sleep(&self, interval: &PollInterval);
}
pub(crate) struct TokioClock;
impl Clock for TokioClock {
    fn now(&self) -> tokio::time::Instant {
        tokio::time::Instant::now()
    }
    async fn sleep(&self, interval: &PollInterval) {
        tokio::time::sleep(interval.duration()).await;
    }
}
pub(crate) async fn poll_ready<C: SandboxControl, K: Clock>(
    control: &C,
    clock: &K,
    id: &SandboxId,
    timeout: &WaitTimeout,
    interval: &PollInterval,
) -> Result<Readiness, OperationError> {
    let end = clock
        .now()
        .checked_add(timeout.duration())
        .ok_or(TransportError::Build)?;
    loop {
        let result = tokio::time::timeout_at(end, control.observe(id)).await;
        match result {
            Err(_) => return Ok(Readiness::Deadline),
            Ok(Err(e)) => return Err(e),
            Ok(Ok(None)) => return Ok(Readiness::Terminal),
            Ok(Ok(Some(observation))) => match observation.state {
                RemoteState::Started => return Ok(Readiness::Ready),
                RemoteState::Destroyed | RemoteState::Failed | RemoteState::Stopped => {
                    return Ok(Readiness::Terminal);
                }
                _ => {}
            },
        }
        if tokio::time::timeout_at(end, clock.sleep(interval))
            .await
            .is_err()
            || clock.now() >= end
        {
            return Ok(Readiness::Deadline);
        }
    }
}
pub(crate) enum Readiness {
    Ready,
    Deadline,
    Terminal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Debug, thiserror::Error)]
    enum TestError {
        #[error(transparent)]
        Validation(#[from] ValidationError),
        #[error(transparent)]
        Operation(#[from] OperationError),
        #[error("polling contract failed")]
        Contract,
    }
    struct PendingControl {
        calls: AtomicUsize,
    }
    impl SandboxControl for PendingControl {
        async fn observe(
            &self,
            _id: &SandboxId,
        ) -> Result<Option<SandboxObservation>, OperationError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(Some(SandboxObservation {
                state: RemoteState::Missing,
                observed_at: std::time::SystemTime::now(),
            }))
        }
    }
    #[tokio::test(start_paused = true)]
    async fn polling_deadline_and_backoff_are_bounded() -> Result<(), TestError> {
        let control = PendingControl {
            calls: AtomicUsize::new(0),
        };
        let result = poll_ready(
            &control,
            &TokioClock,
            &SandboxId::new("sb-1")?,
            &WaitTimeout::seconds(1)?,
            &PollInterval::milliseconds(250)?,
        )
        .await?;
        if !matches!(result, Readiness::Deadline) || control.calls.load(Ordering::Relaxed) != 4 {
            return Err(TestError::Contract);
        }
        Ok(())
    }
    struct HungControl;
    impl SandboxControl for HungControl {
        async fn observe(
            &self,
            _id: &SandboxId,
        ) -> Result<Option<SandboxObservation>, OperationError> {
            std::future::pending().await
        }
    }
    #[tokio::test(start_paused = true)]
    async fn outer_deadline_bounds_a_hung_observation() -> Result<(), TestError> {
        let result = poll_ready(
            &HungControl,
            &TokioClock,
            &SandboxId::new("sb-1")?,
            &WaitTimeout::seconds(1)?,
            &PollInterval::milliseconds(250)?,
        )
        .await?;
        if !matches!(result, Readiness::Deadline) {
            return Err(TestError::Contract);
        }
        Ok(())
    }
}
