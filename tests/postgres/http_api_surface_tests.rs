//! `PostgreSQL` integration tests for the HTTP API surface.

use crate::http_api_test_helpers::{
    HttpApiAuth, assert_shared_error, assert_v1_metadata, required_field, required_str_field,
    with_bearer,
};
use crate::postgres::helpers::BoxError;
use crate::postgres::http_api_surface_common::{PostgresHttpApiContext, TEST_JWT_SECRET, context};
use actix_web::{App, test as actix_test, web};
use corbusier::http_api::api_routes;
use rstest::rstest;
use serde_json::{Value, json};

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_conversation_routes_round_trip(
    #[future] context: Result<PostgresHttpApiContext, BoxError>,
) -> Result<(), BoxError> {
    let postgres_context = context.await?;
    let token = postgres_context.auth.token()?;
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
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_task_routes_round_trip(
    #[future] context: Result<PostgresHttpApiContext, BoxError>,
) -> Result<(), BoxError> {
    let postgres_context = context.await?;
    let token = postgres_context.auth.token()?;
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
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_tool_routes_round_trip(
    #[future] context: Result<PostgresHttpApiContext, BoxError>,
) -> Result<(), BoxError> {
    let postgres_context = context.await?;
    let token = postgres_context.auth.token()?;
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
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_conversation_history_is_tenant_isolated(
    #[future] context: Result<PostgresHttpApiContext, BoxError>,
) -> Result<(), BoxError> {
    let postgres_context = context.await?;
    let owner_token = postgres_context.auth.token()?;
    let other_token = HttpApiAuth::new(TEST_JWT_SECRET).token()?;
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
            &owner_token,
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

    let history_response = actix_test::call_service(
        &app,
        with_bearer(
            actix_test::TestRequest::get()
                .uri(&format!("/api/v1/conversations/{conversation_id}/history")),
            &other_token,
        )
        .to_request(),
    )
    .await;
    assert_eq!(history_response.status().as_u16(), 404);
    let history_body: Value = actix_test::read_body_json(history_response).await;
    assert_shared_error(&history_body, "not_found");
    assert_eq!(
        required_field(required_field(&history_body, "details"), "reason"),
        "conversation_not_found"
    );
    Ok(())
}
