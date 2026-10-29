use daytona::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};

#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Config(#[from] ConfigurationError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    CreateValidation(#[from] CreateValidationError),
    #[error(transparent)]
    Create(#[from] CreateError),
    #[error(transparent)]
    Operation(#[from] OperationError),
    #[error(transparent)]
    Execution(#[from] ExecutionError),
    #[error(transparent)]
    Wait(#[from] WaitError),
    #[error(transparent)]
    Cleanup(#[from] CleanupError),
    #[error(transparent)]
    Transfer(#[from] TransferError),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error(transparent)]
    Elapsed(#[from] tokio::time::error::Elapsed),
    #[error("contract failed: {0}")]
    Contract(&'static str),
}
pub fn require(value: bool, contract: &'static str) -> Result<(), TestError> {
    if value {
        Ok(())
    } else {
        Err(TestError::Contract(contract))
    }
}
pub enum Reply {
    Json(u16, &'static str),
    Disconnect,
    Chunked(&'static str),
    Redirect,
    Stall(std::time::Duration),
    Correlated,
    StallBody(std::time::Duration),
}
pub struct Step {
    pub path: &'static str,
    pub contains: Vec<&'static str>,
    pub reply: Reply,
}
impl Step {
    pub fn json(path: &'static str, status: u16, body: &'static str) -> Self {
        Self {
            path,
            contains: Vec::new(),
            reply: Reply::Json(status, body),
        }
    }
    pub fn containing(mut self, fragments: Vec<&'static str>) -> Self {
        self.contains = fragments;
        self
    }
}
pub struct Server {
    address: std::net::SocketAddr,
    task: JoinHandle<Result<(), TestError>>,
}
impl Server {
    pub async fn start(steps: Vec<Step>) -> Result<Self, TestError> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let task = tokio::spawn(async move {
            for step in steps {
                let (mut stream, _) = listener.accept().await?;
                let mut request = Vec::new();
                let mut buffer = [0_u8; 4096];
                let (header_end, body_len) = loop {
                    let n = stream.read(&mut buffer).await?;
                    require(n != 0, "request terminated before headers")?;
                    request.extend_from_slice(
                        buffer.get(..n).ok_or(TestError::Contract("read range"))?,
                    );
                    require(request.len() <= 65536, "mock request bound")?;
                    if let Some(start) = request.windows(4).position(|s| s == b"\r\n\r\n") {
                        let end = start
                            .checked_add(4)
                            .ok_or(TestError::Contract("header offset"))?;
                        let header = std::str::from_utf8(
                            request
                                .get(..end)
                                .ok_or(TestError::Contract("header range"))?,
                        )
                        .map_err(|_| TestError::Contract("header UTF8"))?;
                        let mut length = 0;
                        for line in header.lines() {
                            if let Some((key, value)) = line.split_once(':')
                                && key.eq_ignore_ascii_case("content-length")
                            {
                                length = value
                                    .trim()
                                    .parse::<usize>()
                                    .map_err(|_| TestError::Contract("content length"))?;
                            }
                        }
                        break (end, length);
                    }
                };
                let total = header_end
                    .checked_add(body_len)
                    .ok_or(TestError::Contract("request size"))?;
                require(total <= 65536, "mock body bound")?;
                while request.len() < total {
                    let n = stream.read(&mut buffer).await?;
                    require(n != 0, "request terminated before body")?;
                    request.extend_from_slice(
                        buffer.get(..n).ok_or(TestError::Contract("body range"))?,
                    );
                }
                let request =
                    String::from_utf8(request).map_err(|_| TestError::Contract("request UTF8"))?;
                require(request.starts_with(step.path), "HTTP method/path")?;
                require(
                    request
                        .to_lowercase()
                        .contains("authorization: bearer test-secret"),
                    "auth header",
                )?;
                for fragment in step.contains {
                    require(request.contains(fragment), "wire field")?;
                }
                match step.reply {
                    Reply::Disconnect => {}
                    Reply::Stall(delay) => tokio::time::sleep(delay).await,
                    Reply::StallBody(delay) => {
                        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n").await?;
                        tokio::time::sleep(delay).await;
                    }
                    Reply::Json(status, body) => {
                        let response = format!(
                            "HTTP/1.1 {status} Mock\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        );
                        stream.write_all(response.as_bytes()).await?;
                    }
                    Reply::Chunked(body) => {
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{body}\r\n0\r\n\r\n",
                            body.len()
                        );
                        stream.write_all(response.as_bytes()).await?;
                    }
                    Reply::Redirect => {
                        let address = stream.local_addr()?;
                        let response = format!(
                            "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://{address}/unexpected-replay\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        );
                        stream.write_all(response.as_bytes()).await?;
                    }
                    Reply::Correlated => {
                        let target = request
                            .split_whitespace()
                            .nth(1)
                            .ok_or(TestError::Contract("request target"))?;
                        let url = url::Url::parse(&format!("http://localhost{target}"))
                            .map_err(|_| TestError::Contract("mock URL"))?;
                        let labels = url
                            .query_pairs()
                            .find(|(key, _)| key == "labels")
                            .ok_or(TestError::Contract("correlation labels"))?
                            .1;
                        let labels: serde_json::Value = serde_json::from_str(&labels)?;
                        let body = format!(
                            "{{\"items\":[{{\"id\":\"sb-1\",\"labels\":{labels}}}],\"nextCursor\":null}}"
                        );
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        );
                        stream.write_all(response.as_bytes()).await?;
                    }
                }
            }
            Ok(())
        });
        Ok(Self { address, task })
    }
    pub fn config(&self) -> Result<ClientConfig, TestError> {
        Ok(ClientConfig::new(
            Endpoint::new(&format!("http://{}/api/", self.address))?,
            Endpoint::new(&format!("http://{}/toolbox/", self.address))?,
        )?
        .request_timeout(RequestTimeout::seconds(2)?)
        .poll_interval(PollInterval::milliseconds(5)?))
    }
    pub fn client(&self) -> Result<Daytona, TestError> {
        Ok(Daytona::with_config(
            Credentials::ApiKey(ApiKey::new("test-secret")?),
            self.config()?,
        )?)
    }
    pub async fn finish(self) -> Result<(), TestError> {
        tokio::time::timeout(std::time::Duration::from_secs(5), self.task).await???;
        Ok(())
    }
}
