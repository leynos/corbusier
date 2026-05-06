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

#[derive(Clone, Copy)]
struct BearerToken<'a>(&'a str);

impl<'a> BearerToken<'a> {
    const fn as_str(self) -> &'a str {
        self.0
    }
}

#[derive(Clone, Copy)]
enum Replacement {
    RequestId,
    Timestamp,
    TaskId,
    TraceId,
}

impl Replacement {
    const fn as_str(self) -> &'static str {
        match self {
            Self::RequestId => "<request-id>",
            Self::Timestamp => "<timestamp>",
            Self::TaskId => "<task-id>",
            Self::TraceId => "<trace-id>",
        }
    }
}

struct ScenarioDesc<'a> {
    label: &'a str,
    body: Value,
    fixture: &'a Utf8Path,
}

struct SuccessExpectation<'a> {
    status: StatusCode,
    scenario: &'a str,
    fixture: &'a Utf8Path,
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_task_contract_matches_golden_fixtures(
    #[future] context: Result<
        crate::postgres::http_api_surface_common::PostgresHttpApiContext,
        BoxError,
    >,
) -> Result<(), BoxError> {
    let postgres_context = context.await?;
    let token_string = postgres_context.auth.token()?;
    let token = BearerToken(token_string.as_str());
    let app = actix_test::init_service(
        App::new()
            .app_data(web::Data::new(postgres_context.state))
            .configure(api_routes),
    )
    .await;

    let (_create_body, task_id) = create_task_succeeds(&app, token).await?;

    let _get_body = run_success_scenario(
        &app,
        with_bearer(
            actix_test::TestRequest::get().uri(&format!("/api/v1/tasks/{task_id}")),
            token.as_str(),
        ),
        SuccessExpectation {
            status: StatusCode::OK,
            scenario: "task get succeeds",
            fixture: Utf8Path::new("tests/fixtures/http_api/tasks/get_success.json"),
        },
    )
    .await?;

    let _transition_body = run_success_scenario(
        &app,
        with_bearer(
            actix_test::TestRequest::put()
                .uri(&format!("/api/v1/tasks/{task_id}/state"))
                .set_json(json!({ "state": "in_progress" })),
            token.as_str(),
        ),
        SuccessExpectation {
            status: StatusCode::OK,
            scenario: "task transition succeeds",
            fixture: Utf8Path::new("tests/fixtures/http_api/tasks/transition_success.json"),
        },
    )
    .await?;

    let scenarios = [
        ScenarioDesc {
            label: "repository validation rejects bad slug",
            body: json!({
                "provider": "github",
                "repository": "bad-repo",
                "issue_number": 42,
                "title": "Implement HTTP API"
            }),
            fixture: Utf8Path::new("tests/fixtures/http_api/tasks/validation_error.json"),
        },
        ScenarioDesc {
            label: "duplicate creation returns conflict envelope",
            body: standard_task_json(),
            fixture: Utf8Path::new("tests/fixtures/http_api/tasks/conflict_error.json"),
        },
    ];
    error_scenario_loop(&app, token, scenarios.as_slice()).await?;

    Ok(())
}

async fn create_task_succeeds<S>(
    app: &S,
    token: BearerToken<'_>,
) -> Result<(Value, String), BoxError>
where
    S: Service<Request, Response = ServiceResponse<BoxBody>, Error = actix_web::Error>,
{
    let create_response = actix_test::call_service(
        app,
        with_bearer(
            actix_test::TestRequest::post()
                .uri("/api/v1/tasks")
                .set_json(standard_task_json()),
            token.as_str(),
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
        Utf8Path::new("tests/fixtures/http_api/tasks/create_success.json"),
    )?;
    Ok((create_body, task_id))
}

async fn run_success_scenario<S>(
    app: &S,
    request: actix_test::TestRequest,
    expectation: SuccessExpectation<'_>,
) -> Result<Value, BoxError>
where
    S: Service<Request, Response = ServiceResponse<BoxBody>, Error = actix_web::Error>,
{
    let response = actix_test::call_service(app, request.to_request()).await;
    assert_actix_status(&response, expectation.status, expectation.scenario)?;
    let mut body: Value = actix_test::read_body_json(response).await;
    normalize_task_success_response(&mut body)?;
    assert_json_matches(&body, expectation.fixture)?;
    Ok(body)
}

async fn error_scenario_loop<S>(
    app: &S,
    token: BearerToken<'_>,
    scenarios: &[ScenarioDesc<'_>],
) -> Result<(), BoxError>
where
    S: Service<Request, Response = ServiceResponse<BoxBody>, Error = actix_web::Error>,
{
    for scenario in scenarios {
        let response = actix_test::call_service(
            app,
            with_bearer(
                actix_test::TestRequest::post()
                    .uri("/api/v1/tasks")
                    .set_json(scenario.body.clone()),
                token.as_str(),
            )
            .to_request(),
        )
        .await;
        assert_actix_status(
            &response,
            expected_http_status(scenario.fixture)?,
            scenario.label,
        )?;
        let mut body: Value = actix_test::read_body_json(response).await;
        normalize_shared_error_response(&mut body)?;
        assert_json_matches(&body, scenario.fixture)?;
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

fn expected_http_status(fixture_path: &Utf8Path) -> Result<StatusCode, BoxError> {
    let path = fixture_path.file_name().ok_or_else(|| {
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

fn assert_json_matches(actual: &Value, expected_path: &Utf8Path) -> Result<(), BoxError> {
    let expected = load_json_fixture(expected_path)?;
    if actual != &expected {
        return Err(Box::new(std::io::Error::other(format!(
            "fixture mismatch for {}\nactual: {actual}\nexpected: {expected}",
            expected_path.as_str()
        ))) as BoxError);
    }
    Ok(())
}

fn load_json_fixture(path: &Utf8Path) -> Result<Value, BoxError> {
    let parent = path.parent().ok_or_else(|| {
        Box::new(std::io::Error::other(format!(
            "fixture path {} should include a parent directory",
            path.as_str()
        ))) as BoxError
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        Box::new(std::io::Error::other(format!(
            "fixture path {} should include a file name",
            path.as_str()
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
    replace_string_at_path(body, &["metadata", "request_id"], Replacement::RequestId)?;
    replace_string_at_path(body, &["metadata", "timestamp"], Replacement::Timestamp)?;
    replace_string_at_path(body, &["data", "task", "id"], Replacement::TaskId)?;
    replace_string_at_path(
        body,
        &["data", "task", "created_at"],
        Replacement::Timestamp,
    )?;
    replace_string_at_path(
        body,
        &["data", "task", "updated_at"],
        Replacement::Timestamp,
    )
}

fn normalize_shared_error_response(body: &mut Value) -> Result<(), BoxError> {
    replace_string_at_path(body, &["traceId"], Replacement::TraceId)
}

fn replace_string_at_path(
    body: &mut Value,
    path: &[&str],
    replacement: Replacement,
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
        Value::String(existing) => replacement.as_str().clone_into(existing),
        other => {
            return Err(Box::new(std::io::Error::other(format!(
                "expected JSON string at leaf `{leaf}` for replacement `{}`, found {other}",
                replacement.as_str()
            ))) as BoxError);
        }
    }
    Ok(())
}
