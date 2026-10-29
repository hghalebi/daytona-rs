use daytona::*;
use serde::{Serialize, de::DeserializeOwned};
#[derive(Debug, thiserror::Error)]
enum TestError {
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Create(#[from] CreateValidationError),
    #[error("contract failed: {0}")]
    Contract(&'static str),
}
fn require(value: bool, contract: &'static str) -> Result<(), TestError> {
    if value {
        Ok(())
    } else {
        Err(TestError::Contract(contract))
    }
}
fn roundtrip<T: Serialize + DeserializeOwned + PartialEq>(value: T) -> Result<(), TestError> {
    let encoded = serde_json::to_string(&value)?;
    let decoded: T = serde_json::from_str(&encoded)?;
    require(value == decoded, "round trip")?;
    require(
        serde_json::to_string(&decoded)? == encoded,
        "stable encoding",
    )
}
#[test]
fn scalar_ingress_and_roundtrips() -> Result<(), TestError> {
    for raw in ["", "../x", "a/b", "a b", "a\n", "☃"] {
        require(SandboxId::new(raw).is_err(), "invalid identity")?;
        require(
            serde_json::from_str::<SandboxId>(&serde_json::to_string(raw)?).is_err(),
            "serde identity ingress",
        )?;
    }
    roundtrip(SandboxId::new("sandbox-123")?)?;
    roundtrip(SessionId::new("session_1")?)?;
    roundtrip(CommandId::new("cmd-1")?)?;
    roundtrip(OrganizationId::new("org-1")?)?;
    roundtrip(SandboxTtl::minutes(10)?)?;
    roundtrip(ExecutionTimeout::seconds(30)?)?;
    roundtrip(IdleDuration::minutes(1)?)?;
    roundtrip(StoppedDuration::minutes(1)?)?;
    roundtrip(CpuAllocation::cores(2)?)?;
    roundtrip(MemorySize::gigabytes(4)?)?;
    roundtrip(DiskSize::gigabytes(8)?)?;
    roundtrip(Port::new(443)?)?;
    require(SandboxTtl::minutes(0).is_err(), "zero TTL")?;
    require(
        SandboxTtl::minutes(u32::MAX).is_err(),
        "provider integer range",
    )?;
    require(
        serde_json::from_str::<SandboxTtl>("0").is_err(),
        "zero TTL hydration",
    )?;
    require(
        serde_json::from_str::<ExecutionTimeout>("0").is_err(),
        "zero timeout hydration",
    )?;
    require(
        serde_json::from_str::<Port>("0").is_err(),
        "zero port hydration",
    )?;
    require(ShellScript::new("\0").is_err(), "nul script")?;
    require(ShellScript::new("\n ").is_err(), "blank script")?;
    roundtrip(ShellScript::new("echo hello\necho world")?)?;
    require(
        ApiKey::new("key\r\nheader").is_err(),
        "credential header injection",
    )?;
    require(ApiKey::new("key value").is_err(), "credential whitespace")?;
    Ok(())
}
#[test]
fn path_network_and_endpoint_boundaries() -> Result<(), TestError> {
    for raw in [
        "relative", "/../etc", "/a/./b", "/a//b", "/a/", "/x\0", "/a\\b",
    ] {
        require(SandboxPath::new(raw).is_err(), "path rejection")?;
        require(
            serde_json::from_str::<SandboxPath>(&serde_json::to_string(raw)?).is_err(),
            "path hydration",
        )?;
    }
    roundtrip(SandboxPath::new("/work/output.txt")?)?;
    roundtrip(SandboxPath::new("/")?)?;
    roundtrip(DomainAllowList::new(vec![
        String::from("*.example.com"),
        String::from("example.org"),
    ])?)?;
    roundtrip(CidrAllowList::new(vec![
        String::from("10.0.0.0/8"),
        String::from("::1/128"),
    ])?)?;
    require(
        DomainAllowList::new(vec![String::from("a,b")]).is_err(),
        "domain delimiter injection",
    )?;
    require(
        CidrAllowList::new(vec![String::from("0.0.0.0/33")]).is_err(),
        "invalid CIDR",
    )?;
    for raw in [
        "http://example.org",
        "https://user:pass@example.org",
        "https://example.org/?secret=a",
        "https://example.org/#secret",
        "file:///tmp",
    ] {
        require(Endpoint::new(raw).is_err(), "endpoint trust grammar")?;
    }
    require(
        Endpoint::new("http://127.0.0.1:8080/api")?
            .as_url()
            .as_str()
            == "http://127.0.0.1:8080/api/",
        "endpoint normalization",
    )?;
    require(
        Endpoint::new(Endpoint::new("https://example.org/api")?.as_url().as_str())?
            .as_url()
            .as_str()
            == "https://example.org/api/",
        "normalization stability",
    )?;
    Ok(())
}
#[test]
fn failed_transition_retains_intent_and_recovery_rehydrates() -> Result<(), TestError> {
    let draft = Create::draft().name(SandboxName::new("retained-name")?);
    let (draft, violation) = match draft.validate() {
        Err(error) => error.into_parts(),
        Ok(_) => return Err(TestError::Contract("missing TTL was accepted")),
    };
    require(
        matches!(violation, ValidationError::MissingTtl),
        "typed validation",
    )?;
    require(
        draft
            .input()
            .name()
            .is_some_and(|name| name.as_str() == "retained-name"),
        "retained input",
    )?;
    let request = draft.ttl(SandboxTtl::minutes(5)?).validate()?;
    require(
        request
            .specification()
            .name()
            .is_some_and(|name| name.as_str() == "retained-name"),
        "corrected intent",
    )?;
    let durable = serde_json::to_string(request.reference())?;
    let restored: CreationReference = serde_json::from_str(&durable)?;
    require(
        restored.operation() == request.reference().operation(),
        "durable correlation",
    )?;
    require(
        serde_json::from_str::<CreationReference>("{\"operation\":\"../bad\"}").is_err(),
        "recovery validates hydration",
    )?;
    Ok(())
}
#[test]
fn secrets_and_intent_redact_debug() -> Result<(), TestError> {
    let secret = "sensitive-secret-canary";
    let credentials = Credentials::Jwt(JwtCredentials::new(
        JwtToken::new(secret)?,
        OrganizationId::new("org")?,
    ));
    let script = ShellScript::new(secret)?;
    let request = Create::draft()
        .source(SandboxSource::Image(ImageSpec::new(secret)?))
        .ttl(SandboxTtl::minutes(1)?)
        .validate()?;
    for debug in [
        format!("{credentials:?}"),
        format!("{script:?}"),
        format!("{request:?}"),
        format!("{:?}", request.reference()),
    ] {
        require(!debug.contains(secret), "nested redaction")?;
    }
    Ok(())
}

#[test]
fn command_budget_validates_every_ingress() -> Result<(), TestError> {
    let budget = CommandBudget::new(ExecutionTimeout::seconds(30)?, RequestTimeout::seconds(5)?);
    let encoded = serde_json::to_string(&budget)?;
    let decoded: CommandBudget = serde_json::from_str(&encoded)?;
    require(
        decoded.execution().as_seconds() == 30 && decoded.http().as_seconds() == 5,
        "independent budget roundtrip",
    )?;
    for invalid in [
        r#"{"execution":0,"http":5}"#,
        r#"{"execution":5,"http":0}"#,
        r#"{"execution":4294967295,"http":5}"#,
    ] {
        require(
            serde_json::from_str::<CommandBudget>(invalid).is_err(),
            "budget ingress",
        )?;
    }
    Ok(())
}
