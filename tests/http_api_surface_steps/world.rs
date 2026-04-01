//! Shared world state for HTTP API behaviour tests.

use actix_web::{App, http::header, test as actix_test, web};
use corbusier::{
    context::RequestContext,
    http_api::{ApiState, BearerTokenAuthenticator, api_routes},
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
use rstest::fixture;
use serde_json::{Value, json};
use std::sync::Arc;
use std::{future::Future, panic::AssertUnwindSafe};

use crate::http_api_test_helpers::HttpApiAuth;

const TEST_JWT_SECRET: &str = "test-http-api-secret";

/// Shared world state for HTTP API behaviour scenarios.
pub struct HttpApiWorld {
    pub state: ApiState,
    pub token: Option<String>,
    pub conversation_id: Option<String>,
    pub task_id: Option<String>,
    pub last_status: Option<u16>,
    pub last_body: Option<Value>,
}

impl HttpApiWorld {
    pub async fn send(
        &mut self,
        test_request: actix_test::TestRequest,
    ) -> Result<(), eyre::Report> {
        let request_with_auth = if let Some(token) = self.token.as_ref() {
            test_request.insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
        } else {
            test_request
        };
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(self.state.clone()))
                .configure(api_routes),
        )
        .await;
        let response = actix_test::call_service(&app, request_with_auth.to_request()).await;
        self.last_status = Some(response.status().as_u16());
        self.last_body = Some(actix_test::read_body_json(response).await);
        Ok(())
    }
}

pub(super) fn required_field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value
        .get(key)
        .unwrap_or_else(|| panic!("expected field `{key}` to be present"))
}

pub(super) fn required_str_field<'a>(value: &'a Value, key: &str) -> &'a str {
    required_field(value, key)
        .as_str()
        .unwrap_or_else(|| panic!("expected field `{key}` to be a string"))
}

fn block_on_setup<F, T>(future: F) -> Result<T, eyre::Report>
where
    F: Future<Output = T>,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        Ok(tokio::task::block_in_place(|| {
            handle.block_on(AssertUnwindSafe(future))
        }))
    } else {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|err| eyre::eyre!("HTTP API test runtime should be created: {err}"))?;
        Ok(runtime.block_on(AssertUnwindSafe(future)))
    }
}

fn build_conversation_service(
    clock: Arc<DefaultClock>,
) -> Arc<
    ConversationService<
        InMemoryConversationRepository,
        InMemoryMessageRepository,
        DefaultMessageValidator,
        DefaultClock,
    >,
> {
    Arc::new(ConversationService::new(
        Arc::new(InMemoryConversationRepository::new()),
        Arc::new(InMemoryMessageRepository::new()),
        Arc::new(DefaultMessageValidator::new()),
        clock,
    ))
}

fn build_task_service(
    clock: Arc<DefaultClock>,
) -> Arc<TaskLifecycleService<InMemoryTaskRepository, DefaultClock>> {
    Arc::new(TaskLifecycleService::new(
        Arc::new(InMemoryTaskRepository::new()),
        clock,
    ))
}

/// Tool infrastructure bundle for test setup.
struct ToolInfrastructure {
    tool_service: Arc<
        ToolDiscoveryRoutingService<
            InMemoryToolCatalog,
            InMemoryMcpServerRegistry,
            InMemoryMcpServerHost,
            AllowAllPolicy,
            ObjectStoreLogAdapter,
            DefaultClock,
        >,
    >,
    lifecycle:
        McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>,
    host: Arc<InMemoryMcpServerHost>,
}

fn build_tool_infrastructure(clock: Arc<DefaultClock>) -> ToolInfrastructure {
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
        clock,
    ));
    ToolInfrastructure {
        tool_service,
        lifecycle,
        host,
    }
}

fn setup_file_tools_server(
    ctx: &RequestContext,
    infrastructure: &ToolInfrastructure,
) -> Result<(), eyre::Report> {
    let server = block_on_setup(async {
        infrastructure
            .lifecycle
            .register(
                ctx,
                RegisterMcpServerRequest::new("file_tools", McpTransport::stdio("echo")?),
            )
            .await
    })??;
    infrastructure
        .host
        .set_tool_catalog(
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
        )
        .map_err(|e| eyre::eyre!("tool catalog should be set: {e}"))?;
    infrastructure
        .host
        .set_tool_call_result(
            server.name().clone(),
            "read_file",
            json!({"content": "hello from tool"}),
        )
        .map_err(|e| eyre::eyre!("tool call result should be set: {e}"))?;
    let _start_result =
        block_on_setup(async { infrastructure.lifecycle.start(ctx, server.id()).await })?;
    let _discover_result = block_on_setup(async {
        infrastructure
            .tool_service
            .discover_and_persist_tools(ctx, server.id())
            .await
    })?;
    Ok(())
}

#[fixture]
pub fn world() -> Result<HttpApiWorld, eyre::Report> {
    let auth = HttpApiAuth::new(TEST_JWT_SECRET);
    let ctx = auth.request_context();
    let clock = Arc::new(DefaultClock);

    let conversation_service = build_conversation_service(clock.clone());
    let task_service = build_task_service(clock.clone());
    let infrastructure = build_tool_infrastructure(clock.clone());

    setup_file_tools_server(&ctx, &infrastructure)?;

    Ok(HttpApiWorld {
        state: ApiState::new(
            conversation_service,
            task_service,
            infrastructure.tool_service,
            BearerTokenAuthenticator::new(TEST_JWT_SECRET),
            clock,
        ),
        token: Some(auth.token()?),
        conversation_id: None,
        task_id: None,
        last_status: None,
        last_body: None,
    })
}
