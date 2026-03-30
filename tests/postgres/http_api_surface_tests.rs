//! `PostgreSQL` integration tests for the HTTP API surface.

use crate::http_api_test_helpers::HttpApiAuth;
use crate::postgres::cluster::TemporaryDatabase;
use crate::postgres::helpers::{
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, postgres_cluster,
};
use actix_web::{App, http::header, test as actix_test, web};
use corbusier::{
    http_api::{ApiState, BearerTokenAuthenticator, api_routes},
    message::{
        adapters::postgres::{PgPool, PostgresConversationRepository, PostgresMessageRepository},
        services::ConversationService,
        validation::service::DefaultMessageValidator,
    },
    task::{adapters::postgres::PostgresTaskRepository, services::TaskLifecycleService},
    tool_registry::{
        adapters::{
            AllowAllPolicy, InMemoryMcpServerHost, ObjectStoreLogAdapter,
            postgres::{PostgresMcpServerRegistry, PostgresToolCatalog},
        },
        domain::{LogRetentionPolicy, McpToolDefinition, McpTransport},
        services::{
            McpServerLifecycleService, RegisterMcpServerRequest, ServicePorts,
            ToolDiscoveryRoutingService,
        },
    },
};
use diesel::{PgConnection, r2d2::ConnectionManager};
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;

const TEST_JWT_SECRET: &str = "test-http-api-secret";

type PostgresToolService = ToolDiscoveryRoutingService<
    PostgresToolCatalog,
    PostgresMcpServerRegistry,
    InMemoryMcpServerHost,
    AllowAllPolicy,
    ObjectStoreLogAdapter,
    DefaultClock,
>;

struct PostgresHttpApiContext {
    state: ApiState,
    auth: HttpApiAuth,
    _temp_db: TemporaryDatabase,
}

fn build_pool(db: &TemporaryDatabase) -> Result<PgPool, BoxError> {
    let manager = ConnectionManager::<PgConnection>::new(db.url());
    diesel::r2d2::Pool::builder()
        .max_size(2)
        .build(manager)
        .map_err(|err| Box::new(err) as BoxError)
}

fn build_state(pool: PgPool) -> Result<(ApiState, HttpApiAuth), BoxError> {
    let auth = HttpApiAuth::new(TEST_JWT_SECRET);
    let ctx = auth.request_context();
    let clock = Arc::new(DefaultClock);

    let conversation_service = Arc::new(ConversationService::new(
        Arc::new(PostgresConversationRepository::new(pool.clone())),
        Arc::new(PostgresMessageRepository::new(pool.clone())),
        Arc::new(DefaultMessageValidator::new()),
        clock.clone(),
    ));
    let task_service = Arc::new(TaskLifecycleService::new(
        Arc::new(PostgresTaskRepository::new(pool.clone())),
        clock.clone(),
    ));
    let tool_service = build_tool_service(pool, &ctx, clock)?;

    Ok((
        ApiState::new(
            conversation_service,
            task_service,
            tool_service,
            BearerTokenAuthenticator::new(TEST_JWT_SECRET),
        ),
        auth,
    ))
}

fn build_tool_service(
    pool: PgPool,
    ctx: &corbusier::context::RequestContext,
    clock: Arc<DefaultClock>,
) -> Result<Arc<PostgresToolService>, BoxError> {
    let registry = Arc::new(PostgresMcpServerRegistry::new(pool.clone()));
    let catalog = Arc::new(PostgresToolCatalog::new(pool));
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

    let server = futures::executor::block_on(async {
        lifecycle
            .register(
                ctx,
                RegisterMcpServerRequest::new("file_tools", McpTransport::stdio("echo")?),
            )
            .await
    })?;
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
    )
    .map_err(|err| Box::new(err) as BoxError)?;
    host.set_tool_call_result(
        server.name().clone(),
        "read_file",
        json!({"content": "hello from tool"}),
    )
    .map_err(|err| Box::new(err) as BoxError)?;
    futures::executor::block_on(async {
        lifecycle
            .start(ctx, server.id())
            .await
            .map_err(|err| Box::new(err) as BoxError)?;
        tool_service
            .discover_and_persist_tools(ctx, server.id())
            .await
            .map_err(|err| Box::new(err) as BoxError)
    })?;

    Ok(tool_service)
}

async fn setup_context(cluster: PostgresCluster) -> Result<PostgresHttpApiContext, BoxError> {
    let db = cluster
        .temporary_database_from_template(&format!("http_api_{}", Uuid::new_v4()), TEMPLATE_DB)
        .await?;
    let pool = build_pool(&db)?;
    let (state, auth) = build_state(pool)?;

    Ok(PostgresHttpApiContext {
        state,
        auth,
        _temp_db: db,
    })
}

#[fixture]
async fn context(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<PostgresHttpApiContext, BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    setup_context(cluster).await
}

fn with_bearer(request: actix_test::TestRequest, token: &str) -> actix_test::TestRequest {
    request.insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
}

fn assert_v1_metadata(body: &Value) {
    let metadata = required_field(body, "metadata");
    assert_eq!(required_field(metadata, "version"), "v1");
    assert!(required_field(metadata, "request_id").is_string());
    assert!(required_field(metadata, "timestamp").is_string());
}

fn required_field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value
        .get(key)
        .unwrap_or_else(|| panic!("expected field `{key}` to be present"))
}

fn required_str_field<'a>(value: &'a Value, key: &str) -> &'a str {
    required_field(value, key)
        .as_str()
        .unwrap_or_else(|| panic!("expected field `{key}` to be a string"))
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_conversation_routes_round_trip(
    #[future] context: Result<PostgresHttpApiContext, BoxError>,
) {
    let postgres_context = context
        .await
        .unwrap_or_else(|err| panic!("postgres context should initialize: {err}"));
    let token = postgres_context
        .auth
        .token()
        .unwrap_or_else(|err| panic!("token encoding should succeed: {err}"));
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(postgres_context.state))
            .configure(api_routes),
    )
    .await;

    let create_response = actix_test::call_service(
        &app,
        with_bearer(
            actix_test::TestRequest::post().uri("/api/v1/conversations"),
            &token,
        )
        .to_request(),
    )
    .await;
    assert_eq!(create_response.status().as_u16(), 201);
    let create_body: Value = actix_test::read_body_json(create_response).await;
    let conversation_id = required_str_field(
        required_field(required_field(&create_body, "data"), "conversation"),
        "id",
    )
    .to_owned();

    let append_response = actix_test::call_service(
        &app,
        with_bearer(
            actix_test::TestRequest::post()
                .uri(&format!("/api/v1/conversations/{conversation_id}/messages"))
                .set_json(json!({
                    "role": "user",
                    "content": [{ "type": "text", "text": "Hello over HTTP" }]
                })),
            &token,
        )
        .to_request(),
    )
    .await;
    assert_eq!(append_response.status().as_u16(), 201);

    let history_response = actix_test::call_service(
        &app,
        with_bearer(
            actix_test::TestRequest::get()
                .uri(&format!("/api/v1/conversations/{conversation_id}/history")),
            &token,
        )
        .to_request(),
    )
    .await;
    assert_eq!(history_response.status().as_u16(), 200);
    let history_body: Value = actix_test::read_body_json(history_response).await;
    assert_v1_metadata(&history_body);
    assert_eq!(
        required_field(required_field(&history_body, "data"), "messages")
            .as_array()
            .map(Vec::len),
        Some(1)
    );
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_task_routes_round_trip(
    #[future] context: Result<PostgresHttpApiContext, BoxError>,
) {
    let postgres_context = context
        .await
        .unwrap_or_else(|err| panic!("postgres context should initialize: {err}"));
    let token = postgres_context
        .auth
        .token()
        .unwrap_or_else(|err| panic!("token encoding should succeed: {err}"));
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(postgres_context.state))
            .configure(api_routes),
    )
    .await;

    let create_response = actix_test::call_service(
        &app,
        with_bearer(
            actix_test::TestRequest::post()
                .uri("/api/v1/tasks")
                .set_json(json!({
                    "provider": "github",
                    "repository": "owner/repo",
                    "issue_number": 42,
                    "title": "Implement HTTP API"
                })),
            &token,
        )
        .to_request(),
    )
    .await;
    assert_eq!(create_response.status().as_u16(), 201);
    let create_body: Value = actix_test::read_body_json(create_response).await;
    let task_id = required_str_field(
        required_field(required_field(&create_body, "data"), "task"),
        "id",
    )
    .to_owned();

    let transition_response = actix_test::call_service(
        &app,
        with_bearer(
            actix_test::TestRequest::put()
                .uri(&format!("/api/v1/tasks/{task_id}/state"))
                .set_json(json!({ "state": "in_progress" })),
            &token,
        )
        .to_request(),
    )
    .await;
    assert_eq!(transition_response.status().as_u16(), 200);
    let transition_body: Value = actix_test::read_body_json(transition_response).await;
    assert_v1_metadata(&transition_body);
    assert_eq!(
        required_field(
            required_field(required_field(&transition_body, "data"), "task"),
            "state"
        ),
        "in_progress"
    );
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_tool_routes_round_trip(
    #[future] context: Result<PostgresHttpApiContext, BoxError>,
) {
    let postgres_context = context
        .await
        .unwrap_or_else(|err| panic!("postgres context should initialize: {err}"));
    let token = postgres_context
        .auth
        .token()
        .unwrap_or_else(|err| panic!("token encoding should succeed: {err}"));
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(postgres_context.state))
            .configure(api_routes),
    )
    .await;

    let list_response = actix_test::call_service(
        &app,
        with_bearer(actix_test::TestRequest::get().uri("/api/v1/tools"), &token).to_request(),
    )
    .await;
    assert_eq!(list_response.status().as_u16(), 200);
    let list_body: Value = actix_test::read_body_json(list_response).await;
    assert_v1_metadata(&list_body);
    assert_eq!(
        required_field(required_field(&list_body, "data"), "tools")
            .as_array()
            .map(Vec::len),
        Some(1)
    );

    let call_response = actix_test::call_service(
        &app,
        with_bearer(
            actix_test::TestRequest::post()
                .uri("/api/v1/tools/calls")
                .set_json(json!({
                    "tool_name": "read_file",
                    "parameters": { "path": "/tmp/example.txt" }
                })),
            &token,
        )
        .to_request(),
    )
    .await;
    assert_eq!(call_response.status().as_u16(), 200);
    let call_body: Value = actix_test::read_body_json(call_response).await;
    assert_v1_metadata(&call_body);
    assert_eq!(
        required_field(required_field(&call_body, "data"), "tool_name"),
        "read_file"
    );
}
