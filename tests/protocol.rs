mod support;
use daytona::*;
use support::*;

const CREATED: &str = r#"{"id":"sb-1","state":"creating"}"#;
const READY: &str = r#"{"id":"sb-1","state":"started"}"#;
const SUCCESS: &str = r#"{"result":"hello\n","exitCode":0}"#;
fn create_step() -> Step {
    Step::json("POST /api/sandbox ", 200, CREATED).containing(vec![
        "\"ttlMinutes\":10",
        "\"autoDeleteInterval\":0",
        "daytona-rs.operation",
        "\"public\":false",
    ])
}
fn ready_step() -> Step {
    Step::json("GET /api/sandbox/sb-1 ", 200, READY)
}
fn execute_step(body: &'static str) -> Step {
    Step::json("POST /toolbox/sb-1/process/execute ", 200, body)
}
fn delete_step(status: u16) -> Step {
    Step::json("DELETE /api/sandbox/sb-1 ", status, "{}")
}
fn absent_step() -> Step {
    Step::json("GET /api/sandbox/sb-1 ", 404, "{}")
}
async fn shell(sandbox: &Sandbox<'_>) -> Result<CheckedOutput, ExecutionError> {
    // The fixture uses a validated script supplied by the test below instead.
    let script = ShellScript::new("echo hello")
        .map_err(|e| OperationError::Protocol(ProtocolError::Validation(e)))?;
    sandbox.shell(script).checked_output().await
}
#[tokio::test]
async fn scoped_workflow_encodes_protocol_and_confirms_deletion() -> Result<(), TestError> {
    let server = Server::start(vec![
        create_step(),
        ready_step(),
        execute_step(SUCCESS).containing(vec!["\"command\":\"echo hello\"", "\"timeout\":30"]),
        delete_step(200),
        absent_step(),
    ])
    .await?;
    let client = server.client()?;
    let script = ShellScript::new("echo hello")?;
    let limit = ExecutionTimeout::seconds(30)?;
    let result = client
        .sandboxes()
        .ephemeral()
        .ttl(SandboxTtl::minutes(10)?)
        .run(async move |sandbox| {
            sandbox
                .shell(script)
                .execution_timeout(limit)
                .checked_output()
                .await
        })
        .await;
    match result {
        Ok(output) => require(output.text() == "hello\n", "owned scope output")?,
        Err(_) => return Err(TestError::Contract("scope success")),
    }
    server.finish().await
}
#[tokio::test]
async fn scoped_errors_preserve_work_cleanup_and_output() -> Result<(), TestError> {
    for (body, status) in [
        (SUCCESS, 500),
        (r#"{"result":"private-output","exitCode":2}"#, 200),
        (r#"{"result":"private-output","exitCode":2}"#, 500),
    ] {
        let mut steps = vec![
            create_step(),
            ready_step(),
            execute_step(body),
            delete_step(status),
        ];
        if status == 200 {
            steps.push(absent_step());
        }
        let server = Server::start(steps).await?;
        let client = server.client()?;
        let result = client
            .sandboxes()
            .ephemeral()
            .ttl(SandboxTtl::minutes(10)?)
            .run_report(async |sandbox| shell(sandbox).await)
            .await;
        match (body == SUCCESS, status, result) {
            (true, 500, Ok(ScopeReport::CleanupFailed { output, cleanup })) => {
                require(
                    output.text() == "hello\n",
                    "output retained on cleanup failure",
                )?;
                require(cleanup.sandbox_id().as_str() == "sb-1", "cleanup identity")?;
            }
            (false, 200, Ok(ScopeReport::WorkFailed(ExecutionError::NonZeroExit { output }))) => {
                require(
                    output.text() == "private-output",
                    "failed command output retained",
                )?
            }
            (false, 500, Ok(ScopeReport::BothFailed { work, cleanup })) => {
                let error = ScopeError::Both { work, cleanup };
                require(
                    error.work_error().is_some() && error.cleanup_error().is_some(),
                    "both failure accessors",
                )?;
                require(
                    !format!("{error:?}").contains("private-output"),
                    "nested output redaction",
                )?;
            }
            _ => return Err(TestError::Contract("scope failure classification")),
        }
        server.finish().await?;
    }
    Ok(())
}
#[tokio::test]
async fn setup_failure_also_attempts_cleanup() -> Result<(), TestError> {
    let server = Server::start(vec![
        create_step(),
        Step::json(
            "GET /api/sandbox/sb-1 ",
            200,
            r#"{"id":"sb-1","state":"build_failed"}"#,
        ),
        delete_step(500),
    ])
    .await?;
    let client = server.client()?;
    let result = client
        .sandboxes()
        .ephemeral()
        .ttl(SandboxTtl::minutes(10)?)
        .run(async |_| Ok::<_, ExecutionError>(()))
        .await;
    match result {
        Err(ScopeError::Setup(ScopeSetupError {
            cause: SetupError::ReadinessTerminal,
            cleanup: Some(cleanup),
        })) => require(
            cleanup.sandbox_id().as_str() == "sb-1",
            "setup cleanup identity",
        )?,
        _ => return Err(TestError::Contract("setup cleanup failure")),
    }
    server.finish().await
}
#[tokio::test]
async fn creation_rejection_and_uncertainty_have_distinct_recovery() -> Result<(), TestError> {
    for status in [400, 401, 403, 409, 429, 408, 500, 502, 307] {
        let server = Server::start(vec![Step::json(
            "POST /api/sandbox ",
            status,
            "secret-provider-body",
        )])
        .await?;
        let client = server.client()?;
        let request = Create::draft().ttl(SandboxTtl::minutes(10)?).validate()?;
        let result = request.submit(&client).await;
        let debug = format!("{result:?}");
        require(
            !debug.contains("test-secret") && !debug.contains("secret-provider-body"),
            "provider error redaction",
        )?;
        match (status, result) {
            (400 | 401 | 403 | 409 | 429, Err(CreateError::Rejected { intent, .. })) => require(
                intent.into_specification().ttl().as_minutes() == 10,
                "rejected intent retained",
            )?,
            (408 | 500 | 502 | 307, Err(CreateError::OutcomeUnknown { recovery, .. })) => require(
                !recovery.reference().operation().as_str().is_empty(),
                "uncertain correlation retained",
            )?,
            _ => return Err(TestError::Contract("create transmission classification")),
        }
        server.finish().await?;
    }
    Ok(())
}
#[tokio::test]
async fn disconnect_and_malformed_acknowledgement_reconcile_without_replay() -> Result<(), TestError>
{
    for reply in [
        Reply::Disconnect,
        Reply::Json(200, "{}"),
        Reply::Json(200, "invalid-json"),
    ] {
        let server = Server::start(vec![
            Step {
                path: "POST /api/sandbox ",
                contains: Vec::new(),
                reply,
            },
            Step {
                path: "GET /api/sandbox?",
                contains: vec![String::from("limit=2")],
                reply: Reply::Correlated,
            },
        ])
        .await?;
        let client = server.client()?;
        let request = Create::draft().ttl(SandboxTtl::minutes(10)?).validate()?;
        let saved = serde_json::to_string(request.reference())?;
        match request.submit(&client).await {
            Err(CreateError::OutcomeUnknown { recovery, .. }) => {
                let restored: CreationReference = serde_json::from_str(&saved)?;
                require(
                    restored.operation() == recovery.reference().operation(),
                    "pre-submit correlation survives failure",
                )?;
                match restored.reconcile(&client).await? {
                    Reconciliation::Found(receipt) => require(
                        receipt.sandbox_id().as_str() == "sb-1",
                        "reconciled identity",
                    )?,
                    Reconciliation::Unresolved => {
                        return Err(TestError::Contract("reconciliation found result"));
                    }
                }
            }
            _ => {
                return Err(TestError::Contract(
                    "ambiguous response must not become replayable",
                ));
            }
        }
        server.finish().await?;
    }
    Ok(())
}
#[tokio::test]
async fn empty_or_ambiguous_reconciliation_does_not_prove_absence() -> Result<(), TestError> {
    for body in [
        r#"{"items":[],"nextCursor":null}"#,
        r#"{"items":[{"id":"sb-1"},{"id":"sb-2"}],"nextCursor":null}"#,
        r#"{"items":[{"id":"sb-1"}],"nextCursor":"page2"}"#,
        r#"{"items":[{"id":"sb-1","labels":{"daytona-rs.operation":"wrong"}}],"nextCursor":null}"#,
    ] {
        let server = Server::start(vec![Step::json("GET /api/sandbox?", 200, body)]).await?;
        let client = server.client()?;
        let request = Create::draft().ttl(SandboxTtl::minutes(1)?).validate()?;
        require(
            matches!(
                request.reference().reconcile(&client).await?,
                Reconciliation::Unresolved
            ),
            "unresolved reference",
        )?;
        server.finish().await?;
    }
    Ok(())
}
#[tokio::test]
async fn missing_exit_and_unknown_remote_state_remain_explicit() -> Result<(), TestError> {
    let server = Server::start(vec![
        execute_step(r#"{"result":"private-output"}"#),
        Step::json(
            "GET /api/sandbox/sb-1 ",
            200,
            r#"{"id":"sb-1","state":"new-provider-state"}"#,
        ),
        Step::json("GET /api/sandbox/sb-1 ", 200, r#"{"id":"sb-1"}"#),
        Step::json(
            "GET /api/sandbox/sb-1 ",
            200,
            r#"{"id":"another","state":"started"}"#,
        ),
    ])
    .await?;
    let client = server.client()?;
    let sandbox = client.sandbox(SandboxId::new("sb-1")?);
    match shell(&sandbox).await {
        Err(ExecutionError::MissingExitStatus { output }) => {
            require(output.exit_code().is_none(), "absent exit remains absent")?
        }
        _ => return Err(TestError::Contract("missing status is not success")),
    }
    match sandbox.observe().await? {
        Some(o) if matches!(o.state(), RemoteState::Unknown(name) if name.as_str() == "new-provider-state") =>
            {}
        _ => return Err(TestError::Contract("unknown remote state preserved")),
    }
    match sandbox.observe().await? {
        Some(o) if matches!(o.state(), RemoteState::Missing) => {}
        _ => return Err(TestError::Contract("missing state preserved")),
    }
    require(
        matches!(
            sandbox.observe().await,
            Err(OperationError::Protocol(ProtocolError::IdentityMismatch))
        ),
        "identity mismatch",
    )?;
    server.finish().await
}
#[tokio::test]
async fn response_capture_is_bounded() -> Result<(), TestError> {
    let server = Server::start(vec![execute_step(SUCCESS)]).await?;
    let config = server.config()?.response_limit(ResponseLimit::new(8)?);
    let client = Daytona::with_config(Credentials::ApiKey(ApiKey::new("test-secret")?), config)?;
    let sandbox = client.sandbox(SandboxId::new("sb-1")?);
    require(
        matches!(
            shell(&sandbox).await,
            Err(ExecutionError::Operation(OperationError::Transport(
                TransportError::BodyLimit
            )))
        ),
        "output limit",
    )?;
    server.finish().await
}
#[tokio::test]
async fn concurrent_borrowed_operations_reject_overload_before_submission() -> Result<(), TestError>
{
    let server = Server::start(vec![create_step()]).await?;
    let client = Daytona::with_config(
        Credentials::ApiKey(ApiKey::new("test-secret")?),
        server.config()?.concurrency(ConcurrencyLimit::new(1)?),
    )?;
    let first = Create::draft().ttl(SandboxTtl::minutes(10)?).validate()?;
    let second = Create::draft().ttl(SandboxTtl::minutes(10)?).validate()?;
    let (a, b) = tokio::join!(first.submit(&client), second.submit(&client));
    match (a, b) {
        (
            Ok(_),
            Err(CreateError::NotSubmitted {
                request,
                cause: OperationError::Transport(TransportError::Busy),
            }),
        ) => require(
            request.specification().ttl().as_minutes() == 10,
            "overload retains retryable request",
        )?,
        _ => return Err(TestError::Contract("one in flight and zero queued")),
    }
    server.finish().await
}
#[tokio::test]
async fn jwt_organization_and_image_resources_are_encoded() -> Result<(), TestError> {
    let server = Server::start(vec![create_step().containing(vec![
        "x-daytona-organization-id: org-1",
        "\"buildInfo\":{\"dockerfileContent\":\"FROM alpine\"}",
        "\"cpu\":2",
        "\"memory\":4",
        "\"disk\":8",
        "\"networkBlockAll\":false",
        "\"domainAllowList\":\"*.example.org\"",
        "\"networkAllowList\":\"10.0.0.0/8\"",
        "\"autoStopInterval\":5",
    ])])
    .await?;
    let client = Daytona::with_config(
        Credentials::Jwt(JwtCredentials::new(
            JwtToken::new("test-secret")?,
            OrganizationId::new("org-1")?,
        )),
        server.config()?,
    )?;
    let receipt = Create::draft()
        .ttl(SandboxTtl::minutes(10)?)
        .source(SandboxSource::Image(ImageSpec::new("FROM alpine")?))
        .resources(Resources::new(
            CpuAllocation::cores(2)?,
            MemorySize::gigabytes(4)?,
            DiskSize::gigabytes(8)?,
        ))
        .network(NetworkPolicy::Allow {
            domains: DomainAllowList::new(vec![String::from("*.example.org")])?,
            cidrs: Some(CidrAllowList::new(vec![String::from("10.0.0.0/8")])?),
        })
        .idle_timeout(IdleDuration::minutes(5)?)
        .validate()?
        .submit(&client)
        .await?;
    require(receipt.sandbox_id().as_str() == "sb-1", "receipt identity")?;
    server.finish().await
}
#[tokio::test]
async fn borrowed_futures_execute_concurrently_without_handle_duplication() -> Result<(), TestError>
{
    let server = Server::start(vec![execute_step(SUCCESS), execute_step(SUCCESS)]).await?;
    let client = server.client()?;
    let sandbox = client.sandbox(SandboxId::new("sb-1")?);
    let first = sandbox
        .shell(ShellScript::new("echo one")?)
        .checked_output();
    let second = sandbox
        .shell(ShellScript::new("echo two")?)
        .checked_output();
    let (a, b) = tokio::join!(first, second);
    require(
        a?.text() == "hello\n" && b?.text() == "hello\n",
        "borrowed concurrent execution",
    )?;
    server.finish().await
}
#[tokio::test]
async fn file_transfer_and_destination_overwrite_protection() -> Result<(), TestError> {
    let server = Server::start(vec![
        Step::json(
            "GET /toolbox/sb-1/files/download?path=%2Fwork%2Ffile ",
            200,
            "file-contents",
        ),
        Step::json(
            "POST /toolbox/sb-1/files/upload-v2?path=%2Fwork%2Ffile ",
            200,
            "{}",
        )
        .containing(vec![
            "file-contents",
            "content-type: application/octet-stream",
        ]),
        Step::json(
            "GET /toolbox/sb-1/files/download?path=%2Fwork%2Ffile ",
            200,
            "changed",
        ),
    ])
    .await?;
    let client = server.client()?;
    let sandbox = client.sandbox(SandboxId::new("sb-1")?);
    let path = std::env::temp_dir().join(format!("daytona-rs-test-{}", uuid::Uuid::new_v4()));
    let local = LocalDestinationPath::new(&path)?;
    let remote = SandboxPath::new("/work/file")?;
    let download = sandbox.download(&remote, &local).await?;
    require(download.bytes().get() == 13, "download byte count")?;
    let upload = sandbox
        .upload(&LocalSourcePath::new(&path)?, &remote)
        .await?;
    require(upload.bytes().get() == 13, "upload byte count")?;
    require(
        matches!(sandbox.download(&remote, &local).await, Err(TransferError::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists),
        "local overwrite forbidden",
    )?;
    require(
        tokio::fs::read_to_string(&path).await? == "file-contents",
        "existing destination preserved",
    )?;
    tokio::fs::remove_file(path).await?;
    server.finish().await
}

#[tokio::test]
async fn creation_timeout_preserves_uncertainty() -> Result<(), TestError> {
    let server = Server::start(vec![Step {
        path: "POST /api/sandbox ",
        contains: Vec::new(),
        reply: Reply::Stall(std::time::Duration::from_millis(1100)),
    }])
    .await?;
    let client = Daytona::with_config(
        Credentials::ApiKey(ApiKey::new("test-secret")?),
        server
            .config()?
            .request_timeout(RequestTimeout::seconds(1)?),
    )?;
    match Create::draft()
        .ttl(SandboxTtl::minutes(1)?)
        .validate()?
        .submit(&client)
        .await
    {
        Err(CreateError::OutcomeUnknown {
            cause: OperationError::Transport(TransportError::Timeout),
            ..
        }) => {}
        _ => return Err(TestError::Contract("timeout is uncertain")),
    }
    server.finish().await
}
#[tokio::test]
async fn wait_deadline_retains_acknowledged_identity() -> Result<(), TestError> {
    let server = Server::start(vec![
        create_step(),
        Step {
            path: "GET /api/sandbox/sb-1 ",
            contains: Vec::new(),
            reply: Reply::Stall(std::time::Duration::from_millis(1100)),
        },
    ])
    .await?;
    let client = server.client()?;
    let receipt = Create::draft()
        .ttl(SandboxTtl::minutes(10)?)
        .validate()?
        .submit(&client)
        .await?;
    match receipt.wait(&client, WaitTimeout::seconds(1)?).await {
        Err(WaitError::Deadline { receipt }) => require(
            receipt.sandbox_id().as_str() == "sb-1",
            "deadline retains receipt",
        )?,
        _ => return Err(TestError::Contract("readiness deadline")),
    }
    server.finish().await
}
#[tokio::test]
async fn accepted_delete_is_not_confirmed_delete() -> Result<(), TestError> {
    let server = Server::start(vec![
        delete_step(200),
        Step {
            path: "GET /api/sandbox/sb-1 ",
            contains: Vec::new(),
            reply: Reply::Stall(std::time::Duration::from_millis(1100)),
        },
    ])
    .await?;
    let client = server.client()?;
    match client
        .sandbox(SandboxId::new("sb-1")?)
        .delete(WaitTimeout::seconds(1)?)
        .await
    {
        Err(CleanupError::Deadline {
            sandbox,
            acceptance: DeleteAcceptance::Acknowledged,
        }) => require(
            sandbox.as_str() == "sb-1",
            "accepted deletion recovery identity",
        )?,
        _ => {
            return Err(TestError::Contract(
                "deletion acknowledgement is not confirmation",
            ));
        }
    }
    server.finish().await
}
#[tokio::test]
async fn cancelled_scope_can_reconcile_from_previously_saved_reference() -> Result<(), TestError> {
    let server = Server::start(vec![
        create_step(),
        ready_step(),
        Step {
            path: "GET /api/sandbox?",
            contains: Vec::new(),
            reply: Reply::Correlated,
        },
        delete_step(200),
        absent_step(),
    ])
    .await?;
    let client = server.client()?;
    let scope = client.sandboxes().ephemeral().ttl(SandboxTtl::minutes(10)?);
    let saved = serde_json::to_string(scope.reference())?;
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        scope.run(async |_| std::future::pending::<Result<(), ExecutionError>>().await),
    )
    .await;
    require(result.is_err(), "scope cancelled during work")?;
    let restored: CreationReference = serde_json::from_str(&saved)?;
    match restored.reconcile(&client).await? {
        Reconciliation::Found(receipt) => {
            client
                .sandbox(receipt.into_id())
                .delete(WaitTimeout::seconds(1)?)
                .await?;
        }
        Reconciliation::Unresolved => return Err(TestError::Contract("cancellation recovery")),
    }
    server.finish().await
}
#[tokio::test]
async fn rejected_work_can_be_recovered_without_leaking_output_in_diagnostics()
-> Result<(), TestError> {
    let server = Server::start(vec![
        create_step(),
        ready_step(),
        execute_step(r#"{"result":"private-output","exitCode":3}"#),
        delete_step(404),
    ])
    .await?;
    let client = server.client()?;
    match client
        .sandboxes()
        .ephemeral()
        .ttl(SandboxTtl::minutes(10)?)
        .run(async |sandbox| shell(sandbox).await)
        .await
    {
        Err(ScopeError::Work(ExecutionError::NonZeroExit { output })) => require(
            output.exit_code().is_some_and(|exit| exit.get() == 3),
            "work failure after cleanup",
        ),
        _ => Err(TestError::Contract("work failure classification")),
    }?;
    server.finish().await
}

#[tokio::test]
async fn cleanup_deadline_also_bounds_delete_submission() -> Result<(), TestError> {
    let server = Server::start(vec![Step {
        path: "DELETE /api/sandbox/sb-1 ",
        contains: Vec::new(),
        reply: Reply::Stall(std::time::Duration::from_millis(1100)),
    }])
    .await?;
    let client = server.client()?;
    match client
        .sandbox(SandboxId::new("sb-1")?)
        .delete(WaitTimeout::seconds(1)?)
        .await
    {
        Err(CleanupError::Deadline {
            acceptance: DeleteAcceptance::Unknown,
            ..
        }) => {}
        _ => return Err(TestError::Contract("cleanup deadline includes submission")),
    }
    server.finish().await
}
#[tokio::test]
async fn incomplete_pagination_response_is_a_protocol_error() -> Result<(), TestError> {
    let server = Server::start(vec![Step::json(
        "GET /api/sandbox?",
        200,
        r#"{"items":[]}"#,
    )])
    .await?;
    let client = server.client()?;
    let request = Create::draft().ttl(SandboxTtl::minutes(1)?).validate()?;
    require(
        matches!(
            request.reference().reconcile(&client).await,
            Err(OperationError::Protocol(ProtocolError::Malformed))
        ),
        "missing pagination metadata is not absence",
    )?;
    server.finish().await
}

#[tokio::test]
async fn chunked_capture_cannot_bypass_the_byte_limit() -> Result<(), TestError> {
    let server = Server::start(vec![Step {
        path: "POST /toolbox/sb-1/process/execute ",
        contains: Vec::new(),
        reply: Reply::Chunked(SUCCESS),
    }])
    .await?;
    let client = Daytona::with_config(
        Credentials::ApiKey(ApiKey::new("test-secret")?),
        server.config()?.response_limit(ResponseLimit::new(8)?),
    )?;
    require(
        matches!(
            shell(&client.sandbox(SandboxId::new("sb-1")?)).await,
            Err(ExecutionError::Operation(OperationError::Transport(
                TransportError::BodyLimit
            )))
        ),
        "chunked response bound",
    )?;
    server.finish().await
}
#[tokio::test]
async fn redirect_does_not_replay_a_creation() -> Result<(), TestError> {
    let server = Server::start(vec![
        Step {
            path: "POST /api/sandbox ",
            contains: Vec::new(),
            reply: Reply::Redirect,
        },
        Step {
            path: "GET /api/sandbox?",
            contains: Vec::new(),
            reply: Reply::Correlated,
        },
    ])
    .await?;
    let client = server.client()?;
    match Create::draft()
        .ttl(SandboxTtl::minutes(1)?)
        .validate()?
        .submit(&client)
        .await
    {
        Err(CreateError::OutcomeUnknown { recovery, .. }) => require(
            matches!(recovery.reconcile(&client).await?, Reconciliation::Found(_)),
            "redirect remains reconcilable",
        )?,
        _ => return Err(TestError::Contract("redirect is not followed")),
    }
    server.finish().await
}
#[tokio::test]
async fn deletion_acknowledgement_does_not_require_capturing_its_body() -> Result<(), TestError> {
    let server = Server::start(vec![
        Step::json(
            "DELETE /api/sandbox/sb-1 ",
            200,
            "body exceeds capture limit",
        ),
        absent_step(),
    ])
    .await?;
    let client = Daytona::with_config(
        Credentials::ApiKey(ApiKey::new("test-secret")?),
        server.config()?.response_limit(ResponseLimit::new(1)?),
    )?;
    client
        .sandbox(SandboxId::new("sb-1")?)
        .delete(WaitTimeout::seconds(1)?)
        .await?;
    server.finish().await
}

#[tokio::test]
async fn command_and_client_deadlines_bound_headers_and_body() -> Result<(), TestError> {
    for (client_seconds, command_seconds, body) in [(1, 3, false), (3, 1, false), (3, 1, true)] {
        let delay = std::time::Duration::from_millis(1200);
        let reply = if body {
            Reply::StallBody(delay)
        } else {
            Reply::Stall(delay)
        };
        let server = Server::start(vec![Step {
            path: "POST /toolbox/sb-1/process/execute ",
            contains: Vec::new(),
            reply,
        }])
        .await?;
        let client = Daytona::with_config(
            Credentials::ApiKey(ApiKey::new("test-secret")?),
            server
                .config()?
                .request_timeout(RequestTimeout::seconds(client_seconds)?),
        )?;
        let sandbox = client.sandbox(SandboxId::new("sb-1")?);
        let result = sandbox
            .shell(ShellScript::new("sleep 10")?)
            .execution_timeout(ExecutionTimeout::seconds(command_seconds)?)
            .output()
            .await;
        require(
            matches!(
                result,
                Err(ExecutionError::Operation(OperationError::Transport(
                    TransportError::Timeout
                )))
            ),
            "earlier HTTP bound covers headers and body",
        )?;
        server.finish().await?;
    }
    Ok(())
}

#[tokio::test]
async fn ownership_helpers_retain_output_identity_and_failure_evidence() -> Result<(), TestError> {
    let server = Server::start(vec![
        execute_step(r#"{"result":"sensitive","exitCode":7}"#),
        execute_step(r#"{"result":"sensitive"}"#),
    ])
    .await?;
    let client = server.client()?;
    let sandbox = client.sandbox(SandboxId::new("sb-1")?);
    for _ in 0..2 {
        let failure = sandbox
            .shell(ShellScript::new("exit 7")?)
            .checked_output()
            .await
            .err()
            .ok_or(TestError::Contract("checked failure"))?;
        require(
            failure.output().is_some_and(|o| o.text() == "sensitive"),
            "output accessor",
        )?;
        require(
            !format!("{failure:?}").contains("sensitive"),
            "output redaction",
        )?;
    }
    require(
        ExecutionError::Operation(TransportError::Busy.into())
            .output()
            .is_none(),
        "no invented output",
    )?;
    let parts = CleanupError::Unconfirmed {
        sandbox: sandbox.into_id(),
        acceptance: DeleteAcceptance::Acknowledged,
        cause: TransportError::Connection.into(),
    }
    .into_parts();
    require(
        parts.sandbox.as_str() == "sb-1"
            && matches!(parts.acceptance, DeleteAcceptance::Acknowledged)
            && matches!(
                parts.reason,
                CleanupReason::Operation(OperationError::Transport(TransportError::Connection))
            ),
        "all cleanup parts",
    )?;
    let parts = CleanupError::Deadline {
        sandbox: parts.sandbox,
        acceptance: parts.acceptance,
    }
    .into_parts();
    require(
        matches!(parts.reason, CleanupReason::Deadline),
        "deadline retained",
    )?;
    server.finish().await
}

#[tokio::test]
async fn callback_deadline_returns_to_scope_for_cleanup() -> Result<(), TestError> {
    let server = Server::start(vec![
        create_step(),
        ready_step(),
        delete_step(200),
        absent_step(),
    ])
    .await?;
    let client = server.client()?;
    let result = client
        .sandboxes()
        .ephemeral()
        .ttl(SandboxTtl::minutes(10)?)
        .run(async |_| {
            tokio::time::timeout(
                std::time::Duration::from_millis(10),
                std::future::pending::<()>(),
            )
            .await
            .map_err(|_| ExecutionError::Operation(TransportError::Timeout.into()))
        })
        .await;
    require(
        matches!(result, Err(ScopeError::Work(_))),
        "callback deadline confirms cleanup",
    )?;
    server.finish().await
}

#[tokio::test]
async fn configured_scopes_preserve_intent_and_all_cleanup_outcomes() -> Result<(), TestError> {
    for (work_succeeds, cleanup_succeeds, ready) in [
        (true, true, true),
        (false, true, true),
        (true, false, true),
        (false, false, true),
        (true, true, false),
    ] {
        let request = Create::draft()
            .ttl(SandboxTtl::minutes(10)?)
            .name(SandboxName::new("configured")?)
            .source(SandboxSource::Snapshot(SnapshotRef::new("snapshot-a")?))
            .resources(Resources::new(
                CpuAllocation::cores(2)?,
                MemorySize::gigabytes(4)?,
                DiskSize::gigabytes(8)?,
            ))
            .network(NetworkPolicy::BlockAll)
            .idle_timeout(IdleDuration::minutes(3)?)
            .delete_after_stop(StoppedDuration::minutes(2)?)
            .validate()?;
        let saved = serde_json::to_string(request.reference())?;
        let mut create = create_step();
        create.contains = vec![serde_json::to_string(request.reference().operation())?];
        for field in [
            "\"name\":\"configured\"",
            "\"snapshot\":\"snapshot-a\"",
            "\"cpu\":2",
            "\"memory\":4",
            "\"disk\":8",
            "\"networkBlockAll\":true",
            "\"autoStopInterval\":3",
            "\"autoDeleteInterval\":2",
            "\"ttlMinutes\":10",
        ] {
            create.contains.push(String::from(field));
        }
        let mut steps = vec![
            create,
            if ready {
                ready_step()
            } else {
                Step::json(
                    "GET /api/sandbox/sb-1 ",
                    200,
                    r#"{"id":"sb-1","state":"stopped"}"#,
                )
            },
        ];
        if ready {
            steps.push(execute_step(if work_succeeds {
                SUCCESS
            } else {
                r#"{"result":"failure","exitCode":7}"#
            }));
        }
        steps.push(delete_step(if cleanup_succeeds { 200 } else { 500 }));
        if cleanup_succeeds {
            steps.push(absent_step());
        }
        let server = Server::start(steps).await?;
        let client = server.client()?;
        let scope = client.sandboxes().ephemeral_from(request);
        require(
            serde_json::to_string(scope.reference())? == saved,
            "exact reference preserved",
        )?;
        let result = scope.run(async |sandbox| shell(sandbox).await).await;
        let valid = match result {
            Ok(_) => ready && work_succeeds && cleanup_succeeds,
            Err(ScopeError::Work(_)) => ready && !work_succeeds && cleanup_succeeds,
            Err(ScopeError::Cleanup(_)) => ready && work_succeeds && !cleanup_succeeds,
            Err(ScopeError::Both { .. }) => ready && !work_succeeds && !cleanup_succeeds,
            Err(ScopeError::Setup(error)) => !ready && error.cleanup.is_none(),
        };
        require(valid, "configured lifecycle outcome")?;
        server.finish().await?;
    }
    Ok(())
}

#[tokio::test]
async fn separate_command_budget_and_setter_precedence() -> Result<(), TestError> {
    let server = Server::start(vec![
        Step {
            path: "POST /toolbox/sb-1/process/execute ",
            contains: vec![String::from("\"timeout\":1")],
            reply: Reply::DelayedOutput(std::time::Duration::from_millis(1200)),
        },
        execute_step(SUCCESS).containing(vec!["\"timeout\":3"]),
    ])
    .await?;
    let client = server.client()?;
    let sandbox = client.sandbox(SandboxId::new("sb-1")?);
    let result = sandbox
        .shell(ShellScript::new("work")?)
        .execution_timeout(ExecutionTimeout::seconds(3)?)
        .budget(CommandBudget::new(
            ExecutionTimeout::seconds(1)?,
            RequestTimeout::seconds(2)?,
        ))
        .checked_output()
        .await?;
    require(
        result.text() == "finished",
        "HTTP grace beyond provider limit",
    )?;
    sandbox
        .shell(ShellScript::new("work")?)
        .budget(CommandBudget::new(
            ExecutionTimeout::seconds(1)?,
            RequestTimeout::seconds(2)?,
        ))
        .execution_timeout(ExecutionTimeout::seconds(3)?)
        .checked_output()
        .await?;
    server.finish().await
}
