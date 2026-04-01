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

        let create_request = with_bearer(
            actix_web::test::TestRequest::post()
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
        let create_response = actix_web::test::call_service(&app, create_request).await;
        eyre::ensure!(
            create_response.status().as_u16() == 201,
            "expected create response status 201, got {}",
            create_response.status().as_u16()
        );
        let create_body: Value = actix_web::test::read_body_json(create_response).await;
        assert_v1_metadata(&create_body);
        let task_id = required_str_field(
            required_field(required_field(&create_body, "data"), "task"),
            "id",
        )
        .to_owned();

        let get_request = with_bearer(
            actix_web::test::TestRequest::get().uri(&format!("/api/v1/tasks/{task_id}")),
            &token,
        )
        .to_request();
        let get_response = actix_web::test::call_service(&app, get_request).await;
        eyre::ensure!(
            get_response.status().as_u16() == 200,
            "expected get response status 200, got {}",
            get_response.status().as_u16()
        );

        let transition_request = with_bearer(
            actix_web::test::TestRequest::put()
                .uri(&format!("/api/v1/tasks/{task_id}/state"))
                .set_json(json!({ "state": "in_progress" })),
            &token,
        )
        .to_request();
        let transition_response = actix_web::test::call_service(&app, transition_request).await;
        eyre::ensure!(
            transition_response.status().as_u16() == 200,
            "expected transition response status 200, got {}",
            transition_response.status().as_u16()
        );
        let transition_body: Value = actix_web::test::read_body_json(transition_response).await;
        eyre::ensure!(
            required_field(
                required_field(required_field(&transition_body, "data"), "task"),
                "state"
            ) == "in_progress",
            "expected transitioned task state to be in_progress"
        );
        assert_v1_metadata(&transition_body);

        Ok(())
    })
}
