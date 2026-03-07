//! In-memory integration tests for agent backend registration and discovery.

use std::sync::Arc;

use super::helpers::ctx;
use corbusier::agent_backend::{
    adapters::memory::InMemoryBackendRegistry,
    domain::BackendStatus,
    ports::BackendRegistryError,
    services::{BackendRegistryService, BackendRegistryServiceError, RegisterBackendRequest},
};
use corbusier::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use mockable::DefaultClock;
use rstest::{fixture, rstest};

type TestService = BackendRegistryService<InMemoryBackendRegistry, DefaultClock>;

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

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn register_two_backends_and_list(service: TestService, ctx: RequestContext) {
    service
        .register(&ctx, claude_request())
        .await
        .expect("first registration should succeed");
    service
        .register(&ctx, codex_request())
        .await
        .expect("second registration should succeed");

    let all = service
        .list_all(&ctx)
        .await
        .expect("listing should succeed");
    assert_eq!(all.len(), 2);

    let names: Vec<&str> = all.iter().map(|b| b.name().as_str()).collect();
    assert!(names.contains(&"claude_code_sdk"));
    assert!(names.contains(&"codex_cli"));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn register_deactivate_list_active_returns_one(service: TestService, ctx: RequestContext) {
    let claude = service
        .register(&ctx, claude_request())
        .await
        .expect("first registration should succeed");
    service
        .register(&ctx, codex_request())
        .await
        .expect("second registration should succeed");

    service
        .deactivate(&ctx, claude.id())
        .await
        .expect("deactivation should succeed");

    let active = service
        .list_active(&ctx)
        .await
        .expect("listing should succeed");
    assert_eq!(active.len(), 1);
    let first = active.first().expect("one entry");
    assert_eq!(first.name().as_str(), "codex_cli");
    assert_eq!(first.status(), BackendStatus::Active);
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn duplicate_name_registration_fails(service: TestService, ctx: RequestContext) {
    service
        .register(&ctx, claude_request())
        .await
        .expect("first registration should succeed");

    let result = service.register(&ctx, claude_request()).await;

    assert!(matches!(
        result,
        Err(BackendRegistryServiceError::Repository(
            BackendRegistryError::DuplicateBackendName(_)
        ))
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn find_by_name_after_registration(service: TestService, ctx: RequestContext) {
    let created = service
        .register(&ctx, claude_request())
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
async fn find_by_name_returns_none_for_unknown(service: TestService, ctx: RequestContext) {
    let found = service
        .find_by_name(&ctx, "nonexistent")
        .await
        .expect("lookup should succeed");

    assert!(found.is_none());
}

// ── Cross-tenant isolation tests ────────────────────────────────────

/// Creates a second `RequestContext` with a distinct `TenantId`.
fn other_ctx() -> RequestContext {
    RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    )
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn register_under_one_tenant_is_invisible_to_another(
    service: TestService,
    ctx: RequestContext,
) {
    let ctx_b = other_ctx();

    service
        .register(&ctx, claude_request())
        .await
        .expect("registration under tenant A should succeed");

    let all_b = service
        .list_all(&ctx_b)
        .await
        .expect("listing under tenant B should succeed");
    assert!(all_b.is_empty(), "tenant B must not see tenant A backends");

    let active_b = service
        .list_active(&ctx_b)
        .await
        .expect("listing active under tenant B should succeed");
    assert!(
        active_b.is_empty(),
        "tenant B must not see tenant A active backends"
    );

    let found_b = service
        .find_by_name(&ctx_b, "claude_code_sdk")
        .await
        .expect("find_by_name under tenant B should succeed");
    assert!(
        found_b.is_none(),
        "tenant B must not find tenant A backend by name"
    );
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn duplicate_name_is_scoped_per_tenant(service: TestService, ctx: RequestContext) {
    let ctx_b = other_ctx();

    service
        .register(&ctx, claude_request())
        .await
        .expect("registration under tenant A should succeed");

    // Same name under a different tenant is allowed.
    service
        .register(&ctx_b, claude_request())
        .await
        .expect("same name under tenant B should succeed");

    let all_a = service
        .list_all(&ctx)
        .await
        .expect("listing under tenant A should succeed");
    assert_eq!(all_a.len(), 1, "tenant A should see exactly one backend");

    let all_b = service
        .list_all(&ctx_b)
        .await
        .expect("listing under tenant B should succeed");
    assert_eq!(all_b.len(), 1, "tenant B should see exactly one backend");
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn deactivate_under_one_tenant_does_not_affect_another(
    service: TestService,
    ctx: RequestContext,
) {
    let ctx_b = other_ctx();

    let backend_a = service
        .register(&ctx, claude_request())
        .await
        .expect("registration under tenant A should succeed");
    service
        .register(&ctx_b, claude_request())
        .await
        .expect("registration under tenant B should succeed");

    service
        .deactivate(&ctx, backend_a.id())
        .await
        .expect("deactivation under tenant A should succeed");

    let active_a = service
        .list_active(&ctx)
        .await
        .expect("listing active under tenant A should succeed");
    assert!(
        active_a.is_empty(),
        "tenant A should have no active backends"
    );

    let active_b = service
        .list_active(&ctx_b)
        .await
        .expect("listing active under tenant B should succeed");
    assert_eq!(active_b.len(), 1, "tenant B backend should remain active");
    let first_b = active_b.first().expect("one entry");
    assert_eq!(first_b.status(), BackendStatus::Active);
}
