//! In-memory integration tests for agent backend registration and discovery.

use std::sync::Arc;

use corbusier::agent_backend::{
    adapters::memory::InMemoryBackendRegistry,
    domain::BackendStatus,
    ports::BackendRegistryError,
    services::{BackendRegistryService, BackendRegistryServiceError, RegisterBackendRequest},
};
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
async fn register_two_backends_and_list(service: TestService) {
    service
        .register(claude_request())
        .await
        .expect("first registration should succeed");
    service
        .register(codex_request())
        .await
        .expect("second registration should succeed");

    let all = service.list_all().await.expect("listing should succeed");
    assert_eq!(all.len(), 2);

    let names: Vec<&str> = all.iter().map(|b| b.name().as_str()).collect();
    assert!(names.contains(&"claude_code_sdk"));
    assert!(names.contains(&"codex_cli"));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn register_deactivate_list_active_returns_one(service: TestService) {
    let claude = service
        .register(claude_request())
        .await
        .expect("first registration should succeed");
    service
        .register(codex_request())
        .await
        .expect("second registration should succeed");

    service
        .deactivate(claude.id())
        .await
        .expect("deactivation should succeed");

    let active = service.list_active().await.expect("listing should succeed");
    assert_eq!(active.len(), 1);
    let first = active.first().expect("one entry");
    assert_eq!(first.name().as_str(), "codex_cli");
    assert_eq!(first.status(), BackendStatus::Active);
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn duplicate_name_registration_fails(service: TestService) {
    service
        .register(claude_request())
        .await
        .expect("first registration should succeed");

    let result = service.register(claude_request()).await;

    assert!(matches!(
        result,
        Err(BackendRegistryServiceError::Repository(
            BackendRegistryError::DuplicateBackendName(_)
        ))
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn find_by_name_after_registration(service: TestService) {
    let created = service
        .register(claude_request())
        .await
        .expect("registration should succeed");

    let found = service
        .find_by_name("claude_code_sdk")
        .await
        .expect("lookup should succeed");

    assert_eq!(found, Some(created));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn find_by_name_returns_none_for_unknown(service: TestService) {
    let found = service
        .find_by_name("nonexistent")
        .await
        .expect("lookup should succeed");

    assert!(found.is_none());
}
