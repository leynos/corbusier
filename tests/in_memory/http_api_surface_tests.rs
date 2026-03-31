//! In-memory integration tests for the HTTP API surface.

use super::helpers::runtime;
use crate::http_api_test_helpers::HttpApiAuth;
use actix_web::{App, http::header, test as actix_test, web};
use corbusier::{
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
use rstest::rstest;
use serde_json::{Value, json};
use std::{io, sync::Arc};
use tokio::runtime::Runtime;

const TEST_JWT_SECRET: &str = "test-http-api-secret";

struct TestBundle {
    state: ApiState,
    auth: HttpApiAuth,
}

async fn build_bundle() -> Result<TestBundle, eyre::Report> {
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
            BearerTokenAuthenticator::new(TEST_JWT_SECRET),
            clock,
        ),
        auth,
    })
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
fn authenticated_conversation_flow(runtime: io::Result<Runtime>) {
    let rt = runtime.unwrap_or_else(|err| panic!("runtime should be available: {err}"));
    rt.block_on(async {
        let bundle = build_bundle()
            .await
            .unwrap_or_else(|err| panic!("bundle setup should succeed: {err}"));
        let token = bundle
            .auth
            .token()
            .unwrap_or_else(|err| panic!("token encoding should succeed: {err}"));
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(bundle.state))
                .configure(api_routes),
        )
        .await;

        let create_request = with_bearer(
            actix_test::TestRequest::post().uri("/api/v1/conversations"),
            &token,
        )
        .to_request();
        let create_response = actix_test::call_service(&app, create_request).await;
        assert_eq!(create_response.status().as_u16(), 201);
        let create_body: Value = actix_test::read_body_json(create_response).await;
        assert_v1_metadata(&create_body);
        let conversation_id = required_str_field(
            required_field(required_field(&create_body, "data"), "conversation"),
            "id",
        )
        .to_owned();

        let append_request = with_bearer(
            actix_test::TestRequest::post()
                .uri(&format!("/api/v1/conversations/{conversation_id}/messages"))
                .set_json(json!({
                    "role": "user",
                    "content": [{ "type": "text", "text": "Hello over HTTP" }]
                })),
            &token,
        )
        .to_request();
        let append_response = actix_test::call_service(&app, append_request).await;
        assert_eq!(append_response.status().as_u16(), 201);
        let append_body: Value = actix_test::read_body_json(append_response).await;
        assert_v1_metadata(&append_body);

        let history_request = with_bearer(
            actix_test::TestRequest::get()
                .uri(&format!("/api/v1/conversations/{conversation_id}/history")),
            &token,
        )
        .to_request();
        let history_response = actix_test::call_service(&app, history_request).await;
        assert_eq!(history_response.status().as_u16(), 200);
        let history_body: Value = actix_test::read_body_json(history_response).await;
        assert_v1_metadata(&history_body);
        assert_eq!(
            required_field(required_field(&history_body, "data"), "messages")
                .as_array()
                .map(Vec::len),
            Some(1)
        );
    });
}

#[rstest]
fn rejects_missing_and_invalid_bearer_tokens(runtime: io::Result<Runtime>) {
    let rt = runtime.unwrap_or_else(|err| panic!("runtime should be available: {err}"));
    rt.block_on(async {
        let bundle = build_bundle()
            .await
            .unwrap_or_else(|err| panic!("bundle setup should succeed: {err}"));
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(bundle.state))
                .configure(api_routes),
        )
        .await;

        let missing_request = actix_test::TestRequest::post()
            .uri("/api/v1/conversations")
            .to_request();
        let missing_response = actix_test::call_service(&app, missing_request).await;
        assert_eq!(missing_response.status().as_u16(), 401);
        let missing_body: Value = actix_test::read_body_json(missing_response).await;
        assert_v1_metadata(&missing_body);

        let invalid_request = with_bearer(
            actix_test::TestRequest::post().uri("/api/v1/conversations"),
            "not-a-jwt",
        )
        .to_request();
        let invalid_response = actix_test::call_service(&app, invalid_request).await;
        assert_eq!(invalid_response.status().as_u16(), 401);
        let invalid_body: Value = actix_test::read_body_json(invalid_response).await;
        assert_v1_metadata(&invalid_body);
    });
}

#[rstest]
fn rejects_unsupported_tenant_kind(runtime: io::Result<Runtime>) {
    let rt = runtime.unwrap_or_else(|err| panic!("runtime should be available: {err}"));
    rt.block_on(async {
        let bundle = build_bundle()
            .await
            .unwrap_or_else(|err| panic!("bundle setup should succeed: {err}"));
        let token = bundle
            .auth
            .token_with_tenant_kind("service")
            .unwrap_or_else(|err| panic!("token encoding should succeed: {err}"));
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(bundle.state))
                .configure(api_routes),
        )
        .await;

        let request = with_bearer(
            actix_test::TestRequest::post().uri("/api/v1/conversations"),
            &token,
        )
        .to_request();
        let response = actix_test::call_service(&app, request).await;
        assert_eq!(response.status().as_u16(), 401);
        let body: Value = actix_test::read_body_json(response).await;
        assert_v1_metadata(&body);
        assert!(
            required_str_field(required_field(&body, "error"), "message")
                .contains("unsupported tenant kind"),
            "error message should mention unsupported tenant kind"
        );
    });
}

#[rstest]
fn rejects_malformed_uuid_claims(runtime: io::Result<Runtime>) {
    let rt = runtime.unwrap_or_else(|err| panic!("runtime should be available: {err}"));
    rt.block_on(async {
        let bundle = build_bundle()
            .await
            .unwrap_or_else(|err| panic!("bundle setup should succeed: {err}"));
        let token = bundle
            .auth
            .token_with_invalid_uuids()
            .unwrap_or_else(|err| panic!("token encoding should succeed: {err}"));
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(bundle.state))
                .configure(api_routes),
        )
        .await;

        let request = with_bearer(
            actix_test::TestRequest::post().uri("/api/v1/conversations"),
            &token,
        )
        .to_request();
        let response = actix_test::call_service(&app, request).await;
        assert_eq!(response.status().as_u16(), 401);
        let body: Value = actix_test::read_body_json(response).await;
        assert_v1_metadata(&body);
    });
}

#[rstest]
fn rejects_expired_tokens(runtime: io::Result<Runtime>) {
    let rt = runtime.unwrap_or_else(|err| panic!("runtime should be available: {err}"));
    rt.block_on(async {
        let bundle = build_bundle()
            .await
            .unwrap_or_else(|err| panic!("bundle setup should succeed: {err}"));
        let token = bundle
            .auth
            .expired_token()
            .unwrap_or_else(|err| panic!("token encoding should succeed: {err}"));
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(bundle.state))
                .configure(api_routes),
        )
        .await;

        let request = with_bearer(
            actix_test::TestRequest::post().uri("/api/v1/conversations"),
            &token,
        )
        .to_request();
        let response = actix_test::call_service(&app, request).await;
        assert_eq!(response.status().as_u16(), 401);
        let body: Value = actix_test::read_body_json(response).await;
        assert_v1_metadata(&body);
    });
}

#[rstest]
fn task_routes_support_create_get_and_transition(runtime: io::Result<Runtime>) {
    let rt = runtime.unwrap_or_else(|err| panic!("runtime should be available: {err}"));
    rt.block_on(async {
        let bundle = build_bundle()
            .await
            .unwrap_or_else(|err| panic!("bundle setup should succeed: {err}"));
        let token = bundle
            .auth
            .token()
            .unwrap_or_else(|err| panic!("token encoding should succeed: {err}"));
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(bundle.state))
                .configure(api_routes),
        )
        .await;

        let create_request = with_bearer(
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
        .to_request();
        let create_response = actix_test::call_service(&app, create_request).await;
        assert_eq!(create_response.status().as_u16(), 201);
        let create_body: Value = actix_test::read_body_json(create_response).await;
        assert_v1_metadata(&create_body);
        let task_id = required_str_field(
            required_field(required_field(&create_body, "data"), "task"),
            "id",
        )
        .to_owned();

        let get_request = with_bearer(
            actix_test::TestRequest::get().uri(&format!("/api/v1/tasks/{task_id}")),
            &token,
        )
        .to_request();
        let get_response = actix_test::call_service(&app, get_request).await;
        assert_eq!(get_response.status().as_u16(), 200);

        let transition_request = with_bearer(
            actix_test::TestRequest::put()
                .uri(&format!("/api/v1/tasks/{task_id}/state"))
                .set_json(json!({ "state": "in_progress" })),
            &token,
        )
        .to_request();
        let transition_response = actix_test::call_service(&app, transition_request).await;
        assert_eq!(transition_response.status().as_u16(), 200);
        let transition_body: Value = actix_test::read_body_json(transition_response).await;
        assert_eq!(
            required_field(
                required_field(required_field(&transition_body, "data"), "task"),
                "state"
            ),
            "in_progress"
        );
        assert_v1_metadata(&transition_body);
    });
}

#[rstest]
fn tool_routes_list_and_call_catalogued_tools(runtime: io::Result<Runtime>) {
    let rt = runtime.unwrap_or_else(|err| panic!("runtime should be available: {err}"));
    rt.block_on(async {
        let bundle = build_bundle()
            .await
            .unwrap_or_else(|err| panic!("bundle setup should succeed: {err}"));
        let token = bundle
            .auth
            .token()
            .unwrap_or_else(|err| panic!("token encoding should succeed: {err}"));
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(bundle.state))
                .configure(api_routes),
        )
        .await;

        let list_request =
            with_bearer(actix_test::TestRequest::get().uri("/api/v1/tools"), &token).to_request();
        let list_response = actix_test::call_service(&app, list_request).await;
        assert_eq!(list_response.status().as_u16(), 200);
        let list_body: Value = actix_test::read_body_json(list_response).await;
        assert_v1_metadata(&list_body);
        assert_eq!(
            required_field(required_field(&list_body, "data"), "tools")
                .as_array()
                .map(Vec::len),
            Some(1)
        );

        let call_request = with_bearer(
            actix_test::TestRequest::post()
                .uri("/api/v1/tools/calls")
                .set_json(json!({
                    "tool_name": "read_file",
                    "parameters": { "path": "/tmp/example.txt" }
                })),
            &token,
        )
        .to_request();
        let call_response = actix_test::call_service(&app, call_request).await;
        assert_eq!(call_response.status().as_u16(), 200);
        let call_body: Value = actix_test::read_body_json(call_response).await;
        assert_v1_metadata(&call_body);
        assert_eq!(
            *required_field(
                required_field(
                    required_field(required_field(&call_body, "data"), "outcome"),
                    "Success"
                ),
                "content"
            ),
            json!({ "content": "hello from tool" })
        );
    });
}
