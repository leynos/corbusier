//! Golden task-contract tests for the in-memory HTTP API surface.

use super::super::helpers::runtime;
use super::support::{build_bundle, with_bearer};
use crate::http_api_test_helpers::{required_field, required_str_field};
use actix_web::{App, test as actix_test, web};
use camino::Utf8Path;
use cap_std::ambient_authority;
use cap_std::fs_utf8::Dir;
use corbusier::http_api::api_routes;
use rstest::rstest;
use serde_json::{Value, json};
use std::io;
use tokio::runtime::Runtime;

#[rstest]
fn task_contract_matches_golden_fixtures(runtime: io::Result<Runtime>) -> Result<(), eyre::Report> {
    let rt = runtime?;
    rt.block_on(async {
        let bundle = build_bundle().await?;
        let token = bundle.auth.token()?;
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(bundle.state))
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
        let mut create_body: Value = actix_test::read_body_json(create_response).await;
        let task_id = required_str_field(
            required_field(required_field(&create_body, "data"), "task"),
            "id",
        )
        .to_owned();
        normalize_task_success_response(&mut create_body)?;
        assert_json_matches(
            &create_body,
            "tests/fixtures/http_api/tasks/create_success.json",
        )?;

        let get_response = actix_test::call_service(
            &app,
            with_bearer(
                actix_test::TestRequest::get().uri(&format!("/api/v1/tasks/{task_id}")),
                &token,
            )
            .to_request(),
        )
        .await;
        let mut get_body: Value = actix_test::read_body_json(get_response).await;
        normalize_task_success_response(&mut get_body)?;
        assert_json_matches(&get_body, "tests/fixtures/http_api/tasks/get_success.json")?;

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
        let mut transition_body: Value = actix_test::read_body_json(transition_response).await;
        normalize_task_success_response(&mut transition_body)?;
        assert_json_matches(
            &transition_body,
            "tests/fixtures/http_api/tasks/transition_success.json",
        )?;

        let validation_response = actix_test::call_service(
            &app,
            with_bearer(
                actix_test::TestRequest::post()
                    .uri("/api/v1/tasks")
                    .set_json(json!({
                        "provider": "github",
                        "repository": "bad-repo",
                        "issue_number": 42,
                        "title": "Implement HTTP API"
                    })),
                &token,
            )
            .to_request(),
        )
        .await;
        let mut validation_body: Value = actix_test::read_body_json(validation_response).await;
        normalize_shared_error_response(&mut validation_body)?;
        assert_json_matches(
            &validation_body,
            "tests/fixtures/http_api/tasks/validation_error.json",
        )?;

        let unauthorized_response = actix_test::call_service(
            &app,
            actix_test::TestRequest::post()
                .uri("/api/v1/tasks")
                .set_json(json!({
                    "provider": "github",
                    "repository": "owner/repo",
                    "issue_number": 42,
                    "title": "Implement HTTP API"
                }))
                .to_request(),
        )
        .await;
        let mut unauthorized_body: Value = actix_test::read_body_json(unauthorized_response).await;
        normalize_shared_error_response(&mut unauthorized_body)?;
        assert_json_matches(
            &unauthorized_body,
            "tests/fixtures/http_api/tasks/unauthorized_error.json",
        )?;

        let not_found_response = actix_test::call_service(
            &app,
            with_bearer(
                actix_test::TestRequest::get()
                    .uri("/api/v1/tasks/11111111-1111-1111-1111-111111111111"),
                &token,
            )
            .to_request(),
        )
        .await;
        let mut not_found_body: Value = actix_test::read_body_json(not_found_response).await;
        normalize_shared_error_response(&mut not_found_body)?;
        assert_json_matches(
            &not_found_body,
            "tests/fixtures/http_api/tasks/not_found_error.json",
        )?;

        let conflict_response = actix_test::call_service(
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
        let mut conflict_body: Value = actix_test::read_body_json(conflict_response).await;
        normalize_shared_error_response(&mut conflict_body)?;
        assert_json_matches(
            &conflict_body,
            "tests/fixtures/http_api/tasks/conflict_error.json",
        )?;

        Ok(())
    })
}

fn assert_json_matches(actual: &Value, expected_path: &str) -> Result<(), eyre::Report> {
    let expected = load_json_fixture(expected_path)?;
    eyre::ensure!(
        actual == &expected,
        "fixture mismatch for {expected_path}\nactual: {actual}\nexpected: {expected}"
    );
    Ok(())
}

fn load_json_fixture(path: &str) -> Result<Value, eyre::Report> {
    let fixture_path = Utf8Path::new(path);
    let parent = fixture_path
        .parent()
        .ok_or_else(|| eyre::eyre!("fixture path {path} should include a parent directory"))?;
    let file_name = fixture_path
        .file_name()
        .ok_or_else(|| eyre::eyre!("fixture path {path} should include a file name"))?;
    let fixture_dir = Dir::open_ambient_dir(parent, ambient_authority())
        .map_err(|err| eyre::eyre!("failed to open fixture directory {parent}: {err}"))?;
    let fixture = fixture_dir
        .read_to_string(file_name)
        .map_err(|err| eyre::eyre!("failed to read fixture {path}: {err}"))?;
    serde_json::from_str(&fixture)
        .map_err(|err| eyre::eyre!("failed to parse fixture {path}: {err}"))
}

fn normalize_task_success_response(body: &mut Value) -> Result<(), eyre::Report> {
    replace_string_at_path(body, &["metadata", "request_id"], "<request-id>")?;
    replace_string_at_path(body, &["metadata", "timestamp"], "<timestamp>")?;
    replace_string_at_path(body, &["data", "task", "id"], "<task-id>")?;
    replace_string_at_path(body, &["data", "task", "created_at"], "<timestamp>")?;
    replace_string_at_path(body, &["data", "task", "updated_at"], "<timestamp>")
}

fn normalize_shared_error_response(body: &mut Value) -> Result<(), eyre::Report> {
    replace_string_at_path(body, &["traceId"], "<trace-id>")
}

fn replace_string_at_path(
    body: &mut Value,
    path: &[&str],
    replacement: &str,
) -> Result<(), eyre::Report> {
    let mut current = body;
    let Some((leaf, parents)) = path.split_last() else {
        return Err(eyre::eyre!("path should not be empty"));
    };
    for segment in parents {
        let Some(next) = current.get_mut(*segment) else {
            return Err(eyre::eyre!("expected path segment `{segment}` to exist"));
        };
        current = next;
    }

    let Some(value) = current.get_mut(*leaf) else {
        return Err(eyre::eyre!("expected path leaf `{leaf}` to exist"));
    };
    *value = Value::String(replacement.to_owned());
    Ok(())
}
