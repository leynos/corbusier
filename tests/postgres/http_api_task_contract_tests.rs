//! Golden task-contract tests for the PostgreSQL-backed HTTP API surface.

use crate::http_api_test_helpers::{required_field, required_str_field, with_bearer};
use crate::postgres::helpers::BoxError;
use crate::postgres::http_api_surface_common::context;
use actix_web::{App, test as actix_test, web};
use camino::Utf8Path;
use cap_std::ambient_authority;
use cap_std::fs_utf8::Dir;
use corbusier::http_api::api_routes;
use rstest::rstest;
use serde_json::{Value, json};

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_task_contract_matches_golden_fixtures(
    #[future] context: Result<
        crate::postgres::http_api_surface_common::PostgresHttpApiContext,
        BoxError,
    >,
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
}

fn assert_json_matches(actual: &Value, expected_path: &str) -> Result<(), BoxError> {
    let expected = load_json_fixture(expected_path)?;
    if actual != &expected {
        return Err(Box::new(std::io::Error::other(format!(
            "fixture mismatch for {expected_path}\nactual: {actual}\nexpected: {expected}"
        ))) as BoxError);
    }
    Ok(())
}

fn load_json_fixture(path: &str) -> Result<Value, BoxError> {
    let fixture_path = Utf8Path::new(path);
    let parent = fixture_path.parent().ok_or_else(|| {
        Box::new(std::io::Error::other(format!(
            "fixture path {path} should include a parent directory"
        ))) as BoxError
    })?;
    let file_name = fixture_path.file_name().ok_or_else(|| {
        Box::new(std::io::Error::other(format!(
            "fixture path {path} should include a file name"
        ))) as BoxError
    })?;
    let fixture_dir = Dir::open_ambient_dir(parent, ambient_authority())
        .map_err(|err| Box::new(err) as BoxError)?;
    let fixture = fixture_dir
        .read_to_string(file_name)
        .map_err(|err| Box::new(err) as BoxError)?;
    serde_json::from_str(&fixture).map_err(|err| Box::new(err) as BoxError)
}

fn normalize_task_success_response(body: &mut Value) -> Result<(), BoxError> {
    replace_string_at_path(body, &["metadata", "request_id"], "<request-id>")?;
    replace_string_at_path(body, &["metadata", "timestamp"], "<timestamp>")?;
    replace_string_at_path(body, &["data", "task", "id"], "<task-id>")?;
    replace_string_at_path(body, &["data", "task", "created_at"], "<timestamp>")?;
    replace_string_at_path(body, &["data", "task", "updated_at"], "<timestamp>")
}

fn normalize_shared_error_response(body: &mut Value) -> Result<(), BoxError> {
    replace_string_at_path(body, &["traceId"], "<trace-id>")
}

fn replace_string_at_path(
    body: &mut Value,
    path: &[&str],
    replacement: &str,
) -> Result<(), BoxError> {
    let mut current = body;
    let Some((leaf, parents)) = path.split_last() else {
        return Err(Box::new(std::io::Error::other("path should not be empty")) as BoxError);
    };
    for segment in parents {
        let Some(next) = current.get_mut(*segment) else {
            return Err(Box::new(std::io::Error::other(format!(
                "expected path segment `{segment}` to exist"
            ))) as BoxError);
        };
        current = next;
    }

    let Some(value) = current.get_mut(*leaf) else {
        return Err(Box::new(std::io::Error::other(format!(
            "expected path leaf `{leaf}` to exist"
        ))) as BoxError);
    };
    *value = Value::String(replacement.to_owned());
    Ok(())
}
