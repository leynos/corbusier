//! Shared world state for HTTP API behaviour tests.

use actix_web::{App, http::header, test as actix_test, web};
use corbusier::{
    context::RequestContext,
    http_api::{ApiConfig, ApiState, BearerTokenAuthenticator, api_routes},
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
use rstest::fixture;
use serde_json::Value;
use std::future::Future;
use std::sync::Arc;

use crate::http_api_test_helpers::{HttpApiAuth, bootstrap_file_tools_server};

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

pub(super) fn world_ref(
    world: &Result<HttpApiWorld, eyre::Report>,
) -> Result<&HttpApiWorld, eyre::Report> {
    world
        .as_ref()
        .map_err(|err| eyre::eyre!("HTTP API world should be constructed: {err}"))
}

pub(super) fn world_mut(
    world: &mut Result<HttpApiWorld, eyre::Report>,
) -> Result<&mut HttpApiWorld, eyre::Report> {
    world
        .as_mut()
        .map_err(|err| eyre::eyre!("HTTP API world should be constructed: {err}"))
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

pub(super) fn required_field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, eyre::Report> {
    value
        .get(key)
        .ok_or_else(|| eyre::eyre!("expected field `{key}` to be present"))
}

pub(super) fn required_str_field<'a>(value: &'a Value, key: &str) -> Result<&'a str, eyre::Report> {
    required_field(value, key).and_then(|field| {
        field
            .as_str()
            .ok_or_else(|| eyre::eyre!("expected field `{key}` to be a string"))
    })
}

fn send_runtime_result<T>(
    sender: &std::sync::mpsc::SyncSender<Result<T, eyre::Report>>,
    result: Result<T, eyre::Report>,
) {
    drop(sender.send(result));
}

fn block_on_setup<F, Fut, T, E>(build_future: F) -> Result<T, eyre::Report>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, E>> + 'static,
    T: Send + 'static,
    E: Into<eyre::Report> + Send + 'static,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let is_multi_thread = matches!(
            handle.runtime_flavor(),
            tokio::runtime::RuntimeFlavor::MultiThread
        ) && handle.metrics().num_workers() > 0;
        if is_multi_thread {
            tokio::task::block_in_place(|| handle.block_on(build_future()).map_err(Into::into))
        } else {
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            std::thread::spawn(move || {
                let result = tokio::runtime::Runtime::new()
                    .map_err(|err| eyre::eyre!("HTTP API test runtime should be created: {err}"))
                    .and_then(|runtime| runtime.block_on(build_future()).map_err(Into::into));
                send_runtime_result(&sender, result);
            });
            receiver
                .recv()
                .map_err(|err| eyre::eyre!("HTTP API test runtime thread should return: {err}"))?
        }
    } else {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|err| eyre::eyre!("HTTP API test runtime should be created: {err}"))?;
        runtime.block_on(build_future()).map_err(Into::into)
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
    lifecycle: Arc<
        McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>,
    >,
    host: Arc<InMemoryMcpServerHost>,
}

fn build_tool_infrastructure(clock: Arc<DefaultClock>) -> ToolInfrastructure {
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
    let request_ctx = ctx.clone();
    let host = infrastructure.host.clone();
    let lifecycle = infrastructure.lifecycle.clone();
    let tool_service = infrastructure.tool_service.clone();
    let register_ctx = request_ctx.clone();
    let start_ctx = request_ctx.clone();
    let discover_ctx = request_ctx;
    let register_lifecycle = lifecycle.clone();
    let start_lifecycle = lifecycle;
    let discover_service = tool_service;
    block_on_setup(move || async move {
        bootstrap_file_tools_server(
            host.as_ref(),
            |request| async move {
                register_lifecycle
                    .register(&register_ctx, request)
                    .await
                    .map_err(eyre::Report::from)
            },
            |server_id| async move {
                start_lifecycle
                    .start(&start_ctx, server_id)
                    .await
                    .map(|_| ())
                    .map_err(eyre::Report::from)
            },
            |server_id| async move {
                discover_service
                    .as_ref()
                    .discover_and_persist_tools(&discover_ctx, server_id)
                    .await
                    .map(|_| ())
                    .map_err(eyre::Report::from)
            },
        )
        .await
    })
}

fn build_world() -> Result<HttpApiWorld, eyre::Report> {
    let auth = HttpApiAuth::new(TEST_JWT_SECRET);
    let ctx = auth.request_context();
    let clock = Arc::new(DefaultClock);

    let conversation_service = build_conversation_service(clock.clone());
    let task_service = build_task_service(clock.clone());
    let infrastructure = build_tool_infrastructure(clock.clone());
    let token = auth.token()?;

    setup_file_tools_server(&ctx, &infrastructure)?;

    Ok(HttpApiWorld {
        state: ApiState::new(
            conversation_service,
            task_service,
            infrastructure.tool_service,
            ApiConfig {
                authenticator: BearerTokenAuthenticator::new(TEST_JWT_SECRET),
                clock: clock as Arc<dyn Clock + Send + Sync>,
            },
        ),
        token: Some(token),
        conversation_id: None,
        task_id: None,
        last_status: None,
        last_body: None,
    })
}

#[fixture]
pub fn world() -> Result<HttpApiWorld, eyre::Report> {
    build_world()
}
