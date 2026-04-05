//! Task route tests for the in-memory HTTP API surface.

use super::super::helpers::runtime;
use super::support::{assert_v1_metadata, build_bundle, with_bearer};
use crate::http_api_test_helpers::{required_field, required_str_field};
use actix_web::{App, web};
use corbusier::http_api::api_routes;
use rstest::rstest;
use serde_json::{Value, json};
use std::io;
use tokio::runtime::Runtime;

async fn create_task<F, Fut, B>(send: F, token: &str) -> Result<String, eyre::Report>
where
    F: FnOnce(actix_web::test::TestRequest) -> Fut,
    Fut: std::future::Future<Output = actix_web::dev::ServiceResponse<B>>,
    B: actix_web::body::MessageBody,
{
    let response = send(with_bearer(
        actix_web::test::TestRequest::post()
            .uri("/api/v1/tasks")
            .set_json(json!({
                "provider": "github",
                "repository": "owner/repo",
                "issue_number": 42,
                "title": "Implement HTTP API"
            })),
        token,
    ))
    .await;
    eyre::ensure!(
        response.status().as_u16() == 201,
        "expected create response status 201, got {}",
        response.status().as_u16()
    );
    let body: Value = actix_web::test::read_body_json(response).await;
    assert_v1_metadata(&body);
    Ok(required_str_field(required_field(required_field(&body, "data"), "task"), "id").to_owned())
}

async fn get_task<F, Fut, B>(send: F, token: &str, task_id: &str) -> Result<String, eyre::Report>
where
    F: FnOnce(actix_web::test::TestRequest) -> Fut,
    Fut: std::future::Future<Output = actix_web::dev::ServiceResponse<B>>,
    B: actix_web::body::MessageBody,
{
    let response = send(with_bearer(
        actix_web::test::TestRequest::get().uri(&format!("/api/v1/tasks/{task_id}")),
        token,
    ))
    .await;
    eyre::ensure!(
        response.status().as_u16() == 200,
        "expected get response status 200, got {}",
        response.status().as_u16()
    );
    let body: Value = actix_web::test::read_body_json(response).await;
    assert_v1_metadata(&body);
    eyre::ensure!(
        required_str_field(required_field(required_field(&body, "data"), "task"), "id") == task_id,
        "expected returned task id to equal requested task id"
    );
    Ok(required_field(required_field(&body, "data"), "task")
        .get("state")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| eyre::eyre!("expected returned task state to be present"))?
        .to_owned())
}

async fn transition_task<F, Fut, B>(send: F, token: &str, task_id: &str) -> Result<(), eyre::Report>
where
    F: FnOnce(actix_web::test::TestRequest) -> Fut,
    Fut: std::future::Future<Output = actix_web::dev::ServiceResponse<B>>,
    B: actix_web::body::MessageBody,
{
    let response = send(with_bearer(
        actix_web::test::TestRequest::put()
            .uri(&format!("/api/v1/tasks/{task_id}/state"))
            .set_json(json!({ "state": "in_progress" })),
        token,
    ))
    .await;
    eyre::ensure!(
        response.status().as_u16() == 200,
        "expected transition response status 200, got {}",
        response.status().as_u16()
    );
    let body: Value = actix_web::test::read_body_json(response).await;
    eyre::ensure!(
        required_field(
            required_field(required_field(&body, "data"), "task"),
            "state"
        ) == "in_progress",
        "expected transitioned task state to be in_progress"
    );
    assert_v1_metadata(&body);
    Ok(())
}

#[rstest]
fn task_routes_support_create_get_and_transition(
    runtime: io::Result<Runtime>,
) -> Result<(), eyre::Report> {
    let rt = runtime?;
    rt.block_on(async {
        let bundle = build_bundle().await?;
        let token = bundle.auth.token()?;
        let app = actix_web::test::init_service(
            App::new()
                .app_data(web::Data::new(bundle.state))
                .configure(api_routes),
        )
        .await;

        let task_id = create_task(
            |request| actix_web::test::call_service(&app, request.to_request()),
            &token,
        )
        .await?;
        let _initial_state = get_task(
            |request| actix_web::test::call_service(&app, request.to_request()),
            &token,
            &task_id,
        )
        .await?;
        transition_task(
            |request| actix_web::test::call_service(&app, request.to_request()),
            &token,
            &task_id,
        )
        .await?;
        let updated_state = get_task(
            |request| actix_web::test::call_service(&app, request.to_request()),
            &token,
            &task_id,
        )
        .await?;
        eyre::ensure!(
            updated_state == "in_progress",
            "expected persisted task state to be in_progress, got {updated_state}"
        );

        Ok(())
    })
}
