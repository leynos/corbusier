//! Golden task-contract tests for the PostgreSQL-backed HTTP API surface.

use crate::http_api_test_helpers::{required_field, required_str_field, with_bearer};
use crate::postgres::helpers::BoxError;
use crate::postgres::http_api_surface_common::context;
use actix_http::Request;
use actix_web::body::BoxBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::StatusCode;
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

    let (_create_body, task_id) = create_task_succeeds(&app, &token).await?;
    let _get_body = get_task_succeeds(&app, &token, &task_id).await?;
    let _transition_body = transition_task_succeeds(&app, &token, &task_id).await?;

    let scenarios = [
        (
            "repository validation rejects bad slug",
            json!({
                "provider": "github",
                "repository": "bad-repo",
                "issue_number": 42,
                "title": "Implement HTTP API"
            }),
            "tests/fixtures/http_api/tasks/validation_error.json",
        ),
        (
            "duplicate creation returns conflict envelope",
            standard_task_json(),
            "tests/fixtures/http_api/tasks/conflict_error.json",
        ),
    ];
    error_scenario_loop(&app, &token, &scenarios).await?;

    Ok(())
}

async fn create_task_succeeds<S>(app: &S, token: &str) -> Result<(Value, String), BoxError>
where
    S: Service<Request, Response = ServiceResponse<BoxBody>, Error = actix_web::Error>,
{
    let create_response = actix_test::call_service(
        app,
        with_bearer(
            actix_test::TestRequest::post()
                .uri("/api/v1/tasks")
                .set_json(standard_task_json()),
            token,
        )
        .to_request(),
    )
    .await;
    assert_actix_status(
        &create_response,
        StatusCode::CREATED,
        "task create succeeds",
    )?;
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
    Ok((create_body, task_id))
}

async fn get_task_succeeds<S>(app: &S, token: &str, task_id: &str) -> Result<Value, BoxError>
where
    S: Service<Request, Response = ServiceResponse<BoxBody>, Error = actix_web::Error>,
{
    let get_response = actix_test::call_service(
        app,
        with_bearer(
            actix_test::TestRequest::get().uri(&format!("/api/v1/tasks/{task_id}")),
            token,
        )
        .to_request(),
    )
    .await;
    assert_actix_status(&get_response, StatusCode::OK, "task get succeeds")?;
    let mut get_body: Value = actix_test::read_body_json(get_response).await;
    normalize_task_success_response(&mut get_body)?;
    assert_json_matches(&get_body, "tests/fixtures/http_api/tasks/get_success.json")?;
    Ok(get_body)
}

async fn transition_task_succeeds<S>(app: &S, token: &str, task_id: &str) -> Result<Value, BoxError>
where
    S: Service<Request, Response = ServiceResponse<BoxBody>, Error = actix_web::Error>,
{
    let transition_response = actix_test::call_service(
        app,
        with_bearer(
            actix_test::TestRequest::put()
                .uri(&format!("/api/v1/tasks/{task_id}/state"))
                .set_json(json!({ "state": "in_progress" })),
            token,
        )
        .to_request(),
    )
    .await;
    assert_actix_status(
        &transition_response,
        StatusCode::OK,
        "task transition succeeds",
    )?;
    let mut transition_body: Value = actix_test::read_body_json(transition_response).await;
    normalize_task_success_response(&mut transition_body)?;
    assert_json_matches(
        &transition_body,
        "tests/fixtures/http_api/tasks/transition_success.json",
    )?;
    Ok(transition_body)
}

async fn error_scenario_loop<S>(
    app: &S,
    token: &str,
    scenarios: &[(&str, Value, &'static str)],
) -> Result<(), BoxError>
where
    S: Service<Request, Response = ServiceResponse<BoxBody>, Error = actix_web::Error>,
{
    for &(description, ref body_json, fixture_path) in scenarios {
        let response = actix_test::call_service(
            app,
            with_bearer(
                actix_test::TestRequest::post()
                    .uri("/api/v1/tasks")
                    .set_json(body_json.clone()),
                token,
            )
            .to_request(),
        )
        .await;
        assert_actix_status(&response, expected_http_status(fixture_path)?, description)?;
        let mut body: Value = actix_test::read_body_json(response).await;
        normalize_shared_error_response(&mut body)?;
        assert_json_matches(&body, fixture_path)?;
    }
    Ok(())
}

fn assert_actix_status(
    response: &ServiceResponse<BoxBody>,
    expected: StatusCode,
    scenario: &str,
) -> Result<(), BoxError> {
    if response.status() != expected {
        return Err(Box::new(std::io::Error::other(format!(
            "{scenario}: expected HTTP {}, got {}",
            expected.as_u16(),
            response.status().as_u16(),
        ))) as BoxError);
    }
    Ok(())
}

fn expected_http_status(fixture_path: &'static str) -> Result<StatusCode, BoxError> {
    let path = Utf8Path::new(fixture_path).file_name().ok_or_else(|| {
        Box::new(std::io::Error::other(format!(
            "fixture path `{fixture_path}` has no final segment"
        ))) as BoxError
    })?;
    Ok(match path {
        "validation_error.json" => StatusCode::BAD_REQUEST,
        "conflict_error.json" => StatusCode::CONFLICT,
        other => {
            return Err(Box::new(std::io::Error::other(format!(
                "fixture `{other}` is not wired to an HTTP status expectation"
            ))) as BoxError);
        }
    })
}

fn standard_task_json() -> Value {
    json!({
        "provider": "github",
        "repository": "owner/repo",
        "issue_number": 42,
        "title": "Implement HTTP API"
    })
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
    match value {
        Value::String(existing) => replacement.clone_into(existing),
        other => {
            return Err(Box::new(std::io::Error::other(format!(
                "expected JSON string at leaf `{leaf}` for replacement `{replacement}`, found {other}"
            ))) as BoxError);
        }
    }
    Ok(())
}
