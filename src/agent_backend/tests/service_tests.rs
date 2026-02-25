//! Unit tests for backend registry service orchestration.

use std::sync::Arc;

use crate::agent_backend::{
    adapters::memory::InMemoryBackendRegistry,
    domain::{BackendDomainError, BackendStatus},
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
    RegisterBackendRequest::new(
        "claude_code_sdk",
        "Claude Code SDK",
        "1.0.0",
        "Anthropic",
        true,
        true,
    )
}

fn codex_request() -> RegisterBackendRequest {
    RegisterBackendRequest::new("codex_cli", "Codex CLI", "0.9.0", "OpenAI", false, true)
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn register_and_retrieve_by_id(service: TestService) {
    let created = service
        .register(claude_request())
        .await
        .expect("registration should succeed");

    let found = service
        .find_by_id(created.id())
        .await
        .expect("lookup should succeed");

    assert_eq!(found, Some(created));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn register_and_retrieve_by_name(service: TestService) {
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
async fn duplicate_name_is_rejected(service: TestService) {
    service
        .register(claude_request())
        .await
        .expect("first registration should succeed");

    let duplicate = service.register(claude_request()).await;

    assert!(matches!(
        duplicate,
        Err(BackendRegistryServiceError::Repository(
            BackendRegistryError::DuplicateBackendName(_)
        ))
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn deactivate_changes_status(service: TestService) {
    let created = service
        .register(claude_request())
        .await
        .expect("registration should succeed");

    let deactivated = service
        .deactivate(created.id())
        .await
        .expect("deactivation should succeed");

    assert_eq!(deactivated.status(), BackendStatus::Inactive);
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn list_active_excludes_inactive(service: TestService) {
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
    assert_eq!(
        active.first().expect("one entry").name().as_str(),
        "codex_cli"
    );
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn list_all_includes_inactive(service: TestService) {
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

    let all = service.list_all().await.expect("listing should succeed");

    assert_eq!(all.len(), 2);
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn find_unknown_id_returns_none(service: TestService) {
    let id = crate::agent_backend::domain::BackendId::new();
    let found = service.find_by_id(id).await.expect("lookup should succeed");
    assert!(found.is_none());
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn invalid_name_in_registration_is_rejected(service: TestService) {
    let request =
        RegisterBackendRequest::new("invalid-name", "Display", "1.0.0", "Provider", true, true);

    let result = service.register(request).await;

    assert!(matches!(
        result,
        Err(BackendRegistryServiceError::Domain(
            BackendDomainError::InvalidBackendName(_)
        ))
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn activate_restores_status(service: TestService) {
    let created = service
        .register(claude_request())
        .await
        .expect("registration should succeed");
    service
        .deactivate(created.id())
        .await
        .expect("deactivation should succeed");

    let activated = service
        .activate(created.id())
        .await
        .expect("activation should succeed");

    assert_eq!(activated.status(), BackendStatus::Active);
}
