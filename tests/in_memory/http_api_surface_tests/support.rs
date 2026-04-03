//! Shared support for in-memory HTTP API surface tests.

use crate::http_api_test_helpers::{HttpApiAuth, bootstrap_file_tools_server};
use actix_web::test as actix_test;
use corbusier::{
    http_api::{ApiConfig, ApiState, BearerTokenAuthenticator},
    message::{
        adapters::memory::{InMemoryConversationRepository, InMemoryMessageRepository},
        services::ConversationService,
        validation::service::DefaultMessageValidator,
    },
    task::{adapters::memory::InMemoryTaskRepository, services::TaskLifecycleService},
    tool_registry::{
        adapters::{
            AllowAllPolicy, InMemoryMcpServerHost, ObjectStoreLogAdapter,
            memory::{InMemoryMcpServerRegistry, InMemoryToolCatalog},
        },
        domain::LogRetentionPolicy,
        services::{McpServerLifecycleService, ServicePorts, ToolDiscoveryRoutingService},
    },
};
use mockable::{Clock, DefaultClock};
use serde_json::Value;
use std::sync::Arc;

pub const TEST_JWT_SECRET: &str = "test-http-api-secret";

pub use crate::http_api_test_helpers::{assert_v1_metadata, with_bearer};

pub struct TestBundle {
    pub state: ApiState,
    pub auth: HttpApiAuth,
}

async fn build_tool_service(
    ctx: &corbusier::context::RequestContext,
    clock: Arc<DefaultClock>,
) -> Result<
    Arc<
        ToolDiscoveryRoutingService<
            InMemoryToolCatalog,
            InMemoryMcpServerRegistry,
            InMemoryMcpServerHost,
            AllowAllPolicy,
            ObjectStoreLogAdapter,
            DefaultClock,
        >,
    >,
    eyre::Report,
> {
    let registry = Arc::new(InMemoryMcpServerRegistry::new());
    let catalog = Arc::new(InMemoryToolCatalog::new());
    let host = Arc::new(InMemoryMcpServerHost::new());
    let lifecycle = Arc::new(McpServerLifecycleService::new(
        registry.clone(),
        host.clone(),
        clock.clone(),
    ));
    let tool_service = Arc::new(ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog,
            registry: registry.clone(),
            host: host.clone(),
            governance: Arc::new(AllowAllPolicy::new()),
            log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
        },
        LogRetentionPolicy::default(),
        clock.clone(),
    ));
    let register_ctx = ctx.clone();
    let start_ctx = ctx.clone();
    let discover_ctx = ctx.clone();
    let register_lifecycle = lifecycle.clone();
    let start_lifecycle = lifecycle;
    let discover_service = tool_service.clone();

    bootstrap_file_tools_server(
        host.as_ref(),
        |request| async move {
            register_lifecycle
                .as_ref()
                .register(&register_ctx, request)
                .await
                .map_err(eyre::Report::from)
        },
        |server_id| async move {
            start_lifecycle
                .as_ref()
                .start(&start_ctx, server_id)
                .await
                .map(|_| ())
                .map_err(eyre::Report::from)
        },
        |server_id| async move {
            discover_service
                .discover_and_persist_tools(&discover_ctx, server_id)
                .await
                .map(|_| ())
                .map_err(eyre::Report::from)
        },
    )
    .await?;

    Ok(tool_service)
}

pub async fn build_bundle() -> Result<TestBundle, eyre::Report> {
    let auth = HttpApiAuth::new(TEST_JWT_SECRET);
    let ctx = auth.request_context();
    let clock = Arc::new(DefaultClock);

    let conversation_service = Arc::new(ConversationService::new(
        Arc::new(InMemoryConversationRepository::new()),
        Arc::new(InMemoryMessageRepository::new()),
        Arc::new(DefaultMessageValidator::new()),
        clock.clone(),
    ));

    let task_service = Arc::new(TaskLifecycleService::new(
        Arc::new(InMemoryTaskRepository::new()),
        clock.clone(),
    ));
    let tool_service = build_tool_service(&ctx, clock.clone()).await?;

    Ok(TestBundle {
        state: ApiState::new(
            conversation_service,
            task_service,
            tool_service,
            ApiConfig {
                authenticator: BearerTokenAuthenticator::new(TEST_JWT_SECRET),
                clock: clock as Arc<dyn Clock + Send + Sync>,
            },
        ),
        auth,
    })
}

pub async fn assert_rejects_response<B>(response: actix_web::dev::ServiceResponse<B>) -> Value
where
    B: actix_web::body::MessageBody,
{
    assert_eq!(response.status().as_u16(), 401);
    let body: Value = actix_test::read_body_json(response).await;
    assert_v1_metadata(&body);
    body
}

pub fn reject_conversations_request(token: &str) -> actix_test::TestRequest {
    with_bearer(
        actix_test::TestRequest::post().uri("/api/v1/conversations"),
        token,
    )
}
