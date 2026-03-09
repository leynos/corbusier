//! Unit tests for backend registry service orchestration.

use std::sync::Arc;

use crate::agent_backend::{
    adapters::memory::InMemoryBackendRegistry,
    domain::{AgentBackendRegistration, BackendDomainError, BackendStatus},
    ports::BackendRegistryError,
    services::{BackendRegistryService, BackendRegistryServiceError, RegisterBackendRequest},
};
use crate::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use mockable::DefaultClock;
use rstest::{fixture, rstest};

type TestService = BackendRegistryService<InMemoryBackendRegistry, DefaultClock>;

#[fixture]
fn ctx() -> RequestContext {
    RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    )
}

#[fixture]
fn service() -> TestService {
    BackendRegistryService::new(
        Arc::new(InMemoryBackendRegistry::new()),
        Arc::new(DefaultClock),
    )
}

fn claude_request() -> RegisterBackendRequest {
    RegisterBackendRequest::new("claude_code_sdk", "Claude Code SDK", "1.0.0", "Anthropic")
        .with_capabilities(true, true)
}

fn codex_request() -> RegisterBackendRequest {
    RegisterBackendRequest::new("codex_cli", "Codex CLI", "0.9.0", "OpenAI")
        .with_capabilities(false, true)
}

async fn register_backend(
    service: &TestService,
    ctx: &RequestContext,
    request: RegisterBackendRequest,
) -> Result<AgentBackendRegistration, BackendRegistryServiceError> {
    service.register(ctx, request).await
}

async fn register_and_deactivate(
    service: &TestService,
    ctx: &RequestContext,
    request: RegisterBackendRequest,
) -> Result<AgentBackendRegistration, BackendRegistryServiceError> {
    let created = register_backend(service, ctx, request).await?;
    service.deactivate(ctx, created.id()).await
}

async fn setup_active_and_inactive(
    service: &TestService,
    ctx: &RequestContext,
) -> Result<(), BackendRegistryServiceError> {
    let claude = register_backend(service, ctx, claude_request()).await?;
    register_backend(service, ctx, codex_request()).await?;
    service.deactivate(ctx, claude.id()).await?;
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn register_and_retrieve_by_id(service: TestService, ctx: RequestContext) {
    let created = register_backend(&service, &ctx, claude_request())
        .await
        .expect("registration should succeed");

    let found = service
        .find_by_id(&ctx, created.id())
        .await
        .expect("lookup should succeed");

    assert_eq!(found, Some(created));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn register_and_retrieve_by_name(service: TestService, ctx: RequestContext) {
    let created = register_backend(&service, &ctx, claude_request())
        .await
        .expect("registration should succeed");

    let found = service
        .find_by_name(&ctx, "claude_code_sdk")
        .await
        .expect("lookup should succeed");

    assert_eq!(found, Some(created));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn duplicate_name_is_rejected(service: TestService, ctx: RequestContext) {
    service
        .register(&ctx, claude_request())
        .await
        .expect("first registration should succeed");

    let duplicate = service.register(&ctx, claude_request()).await;

    assert!(matches!(
        duplicate,
        Err(BackendRegistryServiceError::Repository(
            BackendRegistryError::DuplicateBackendName(_)
        ))
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn deactivate_changes_status(service: TestService, ctx: RequestContext) {
    let deactivated = register_and_deactivate(&service, &ctx, claude_request())
        .await
        .expect("register and deactivate should succeed");

    assert_eq!(deactivated.status(), BackendStatus::Inactive);
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn list_active_excludes_inactive(service: TestService, ctx: RequestContext) {
    setup_active_and_inactive(&service, &ctx)
        .await
        .expect("setup should succeed");

    let active = service
        .list_active(&ctx)
        .await
        .expect("listing should succeed");

    assert_eq!(active.len(), 1);
    assert_eq!(
        active.first().expect("one entry").name().as_str(),
        "codex_cli"
    );
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn list_all_includes_inactive(service: TestService, ctx: RequestContext) {
    setup_active_and_inactive(&service, &ctx)
        .await
        .expect("setup should succeed");

    let all = service
        .list_all(&ctx)
        .await
        .expect("listing should succeed");

    assert_eq!(all.len(), 2);
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn find_unknown_id_returns_none(service: TestService, ctx: RequestContext) {
    let id = crate::agent_backend::domain::BackendId::new();
    let found = service
        .find_by_id(&ctx, id)
        .await
        .expect("lookup should succeed");
    assert!(found.is_none());
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn invalid_name_in_registration_is_rejected(service: TestService, ctx: RequestContext) {
    let request = RegisterBackendRequest::new("invalid-name", "Display", "1.0.0", "Provider")
        .with_capabilities(true, true);

    let result = service.register(&ctx, request).await;

    assert!(matches!(
        result,
        Err(BackendRegistryServiceError::Domain(
            BackendDomainError::InvalidBackendName(_)
        ))
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn activate_restores_status(service: TestService, ctx: RequestContext) {
    let deactivated = register_and_deactivate(&service, &ctx, claude_request())
        .await
        .expect("register and deactivate should succeed");

    let activated = service
        .activate(&ctx, deactivated.id())
        .await
        .expect("activation should succeed");

    assert_eq!(activated.status(), BackendStatus::Active);
}
