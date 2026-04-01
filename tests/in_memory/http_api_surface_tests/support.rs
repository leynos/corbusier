//! Shared support for in-memory HTTP API surface tests.

use crate::http_api_test_helpers::HttpApiAuth;
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
        domain::{LogRetentionPolicy, McpToolDefinition, McpTransport},
        services::{
            McpServerLifecycleService, RegisterMcpServerRequest, ServicePorts,
            ToolDiscoveryRoutingService,
        },
    },
};
use mockable::DefaultClock;
use serde_json::{Value, json};
use std::sync::Arc;

pub const TEST_JWT_SECRET: &str = "test-http-api-secret";

pub use crate::http_api_test_helpers::{assert_v1_metadata, with_bearer};

pub struct TestBundle {
    pub state: ApiState,
    pub auth: HttpApiAuth,
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

    let registry = Arc::new(InMemoryMcpServerRegistry::new());
    let catalog = Arc::new(InMemoryToolCatalog::new());
    let host = Arc::new(InMemoryMcpServerHost::new());
    let lifecycle = McpServerLifecycleService::new(registry.clone(), host.clone(), clock.clone());
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

    let server = lifecycle
        .register(
            &ctx,
            RegisterMcpServerRequest::new("file_tools", McpTransport::stdio("echo")?),
        )
        .await?;
    host.set_tool_catalog(
        server.name().clone(),
        vec![McpToolDefinition::new(
            "read_file",
            "Read a file from disk",
            json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "required": ["path"]
            }),
        )?],
    )?;
    host.set_tool_call_result(
        server.name().clone(),
        "read_file",
        json!({"content": "hello from tool"}),
    )?;
    lifecycle.start(&ctx, server.id()).await?;
    tool_service
        .discover_and_persist_tools(&ctx, server.id())
        .await?;

    Ok(TestBundle {
        state: ApiState::new(
            conversation_service,
            task_service,
            tool_service,
            ApiConfig {
                authenticator: BearerTokenAuthenticator::new(TEST_JWT_SECRET),
                clock,
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
