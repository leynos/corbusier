//! Golden task-contract tests for the in-memory HTTP API surface.

use super::super::helpers::runtime;
use super::support::{build_bundle, with_bearer};
use crate::http_api_test_helpers::{required_field, required_str_field};
use actix_web::body::BoxBody;
use actix_web::dev::ServiceResponse;
use actix_web::http::StatusCode;
use actix_web::{App, test as actix_test, web};
use camino::Utf8Path;
use cap_std::ambient_authority;
use cap_std::fs_utf8::Dir;
use corbusier::http_api::api_routes;
use rstest::rstest;
use serde_json::{Value, json};
use std::future::Future;
use std::io;
use std::pin::Pin;
use tokio::runtime::Runtime;

/// Opaque bearer token used in test requests.
#[derive(Clone, Copy)]
struct BearerToken<'a>(&'a str);

impl<'a> BearerToken<'a> {
    const fn as_str(self) -> &'a str {
        self.0
    }
}

/// Sentinel placeholder written over dynamic fields before fixture comparison.
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

/// Type-erased sender: drives a single actix test request through the service
/// and returns a boxed response. The concrete alias removes the `<F, Fut, B>`
/// generic triple - and its identical `where` clause - from every `check_*`
/// helper.
type Sender<'a> = &'a dyn Fn(
    actix_test::TestRequest,
) -> Pin<Box<dyn Future<Output = ServiceResponse<BoxBody>> + 'a>>;

#[expect(
    clippy::too_many_arguments,
    reason = "repository threshold is stricter than Clippy's default and the test scenarios pass five stable parts"
)]
async fn run_scenario<Extract, Out>(
    request: actix_test::TestRequest,
    send: Sender<'_>,
    extract: Extract,
    normalise: fn(&mut Value) -> Result<(), eyre::Report>,
    fixture: &Utf8Path,
    expected_status: StatusCode,
) -> Result<Out, eyre::Report>
where
    Extract: FnOnce(&Value) -> Result<Out, eyre::Report>,
{
    let response = send(request).await;
    if response.status() != expected_status {
        return Err(eyre::eyre!(
            "expected HTTP {:?} {:?}, got {:?} {:?}",
            expected_status,
            expected_status.canonical_reason(),
            response.status(),
            response.status().canonical_reason(),
        ));
    }
    let mut body: Value = actix_test::read_body_json(response).await;
    let out = extract(&body)?;
    normalise(&mut body)?;
    assert_json_matches(&body, fixture)?;
    Ok(out)
}

fn create_task_request(token: BearerToken<'_>) -> actix_test::TestRequest {
    with_bearer(
        actix_test::TestRequest::post()
            .uri("/api/v1/tasks")
            .set_json(standard_task_json()),
        token.as_str(),
    )
}

fn get_task_request(task_id: &str, token: BearerToken<'_>) -> actix_test::TestRequest {
    with_bearer(
        actix_test::TestRequest::get().uri(&format!("/api/v1/tasks/{task_id}")),
        token.as_str(),
    )
}

fn transition_task_request(task_id: &str, token: BearerToken<'_>) -> actix_test::TestRequest {
    with_bearer(
        actix_test::TestRequest::put()
            .uri(&format!("/api/v1/tasks/{task_id}/state"))
            .set_json(json!({ "state": "in_progress" })),
        token.as_str(),
    )
}

fn validation_error_request(token: BearerToken<'_>) -> actix_test::TestRequest {
    with_bearer(
        actix_test::TestRequest::post()
            .uri("/api/v1/tasks")
            .set_json(json!({
                "provider": "github",
                "repository": "bad-repo",
                "issue_number": 42,
                "title": "Implement HTTP API"
            })),
        token.as_str(),
    )
}

fn malformed_idempotency_key_request(token: BearerToken<'_>) -> actix_test::TestRequest {
    with_bearer(
        actix_test::TestRequest::post()
            .uri("/api/v1/tasks")
            .insert_header(("Idempotency-Key", "not-a-valid-uuid"))
            .set_json(standard_task_json()),
        token.as_str(),
    )
}

fn unauthorized_error_request() -> actix_test::TestRequest {
    actix_test::TestRequest::post()
        .uri("/api/v1/tasks")
        .set_json(standard_task_json())
}

fn not_found_error_request(token: BearerToken<'_>) -> actix_test::TestRequest {
    with_bearer(
        actix_test::TestRequest::get().uri("/api/v1/tasks/11111111-1111-1111-1111-111111111111"),
        token.as_str(),
    )
}

fn conflict_error_request(token: BearerToken<'_>) -> actix_test::TestRequest {
    create_task_request(token)
}

fn task_id_from_body(body: &Value) -> String {
    required_str_field(required_field(required_field(body, "data"), "task"), "id").to_owned()
}

fn fixture(path: &'static str) -> &'static Utf8Path {
    Utf8Path::new(path)
}

fn standard_task_json() -> serde_json::Value {
    json!({
        "provider": "github",
        "repository": "owner/repo",
        "issue_number": 42,
        "title": "Implement HTTP API"
    })
}

async fn run_happy_path_scenarios(
    send: Sender<'_>,
    token: BearerToken<'_>,
) -> Result<String, eyre::Report> {
    let task_id: String = run_scenario(
        create_task_request(token),
        send,
        |body| Ok(task_id_from_body(body)),
        normalize_task_success_response,
        fixture("tests/fixtures/http_api/tasks/create_success.json"),
        StatusCode::CREATED,
    )
    .await?;
    run_scenario(
        get_task_request(&task_id, token),
        send,
        |_| Ok(()),
        normalize_task_success_response,
        fixture("tests/fixtures/http_api/tasks/get_success.json"),
        StatusCode::OK,
    )
    .await?;
    run_scenario(
        transition_task_request(&task_id, token),
        send,
        |_| Ok(()),
        normalize_task_success_response,
        fixture("tests/fixtures/http_api/tasks/transition_success.json"),
        StatusCode::OK,
    )
    .await?;
    Ok(task_id)
}

async fn run_error_scenarios(send: Sender<'_>, token: BearerToken<'_>) -> Result<(), eyre::Report> {
    let scenarios: [(actix_test::TestRequest, &Utf8Path, StatusCode); 5] = [
        (
            validation_error_request(token),
            fixture("tests/fixtures/http_api/tasks/validation_error.json"),
            StatusCode::BAD_REQUEST,
        ),
        (
            malformed_idempotency_key_request(token),
            fixture("tests/fixtures/http_api/tasks/invalid_idempotency_key.json"),
            StatusCode::BAD_REQUEST,
        ),
        (
            unauthorized_error_request(),
            fixture("tests/fixtures/http_api/tasks/unauthorized_error.json"),
            StatusCode::UNAUTHORIZED,
        ),
        (
            not_found_error_request(token),
            fixture("tests/fixtures/http_api/tasks/not_found_error.json"),
            StatusCode::NOT_FOUND,
        ),
        (
            conflict_error_request(token),
            fixture("tests/fixtures/http_api/tasks/conflict_error.json"),
            StatusCode::CONFLICT,
        ),
    ];
    for (request, path, expected_status) in scenarios {
        run_scenario(
            request,
            send,
            |_| Ok(()),
            normalize_shared_error_response,
            path,
            expected_status,
        )
        .await?;
    }
    Ok(())
}

#[rstest]
fn task_contract_matches_golden_fixtures(runtime: io::Result<Runtime>) -> Result<(), eyre::Report> {
    let rt = runtime?;
    rt.block_on(async {
        let bundle = build_bundle().await?;
        let token_str = bundle.auth.token()?;
        let token = BearerToken(token_str.as_str());
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(bundle.state))
                .configure(api_routes),
        )
        .await;
        let send_fn =
            |req: actix_test::TestRequest| -> Pin<Box<dyn Future<Output = ServiceResponse<BoxBody>> + '_>> {
                Box::pin(async {
                    actix_test::call_service(&app, req.to_request())
                        .await
                        .map_into_boxed_body()
                })
            };
        let send: Sender<'_> = &send_fn;

        run_happy_path_scenarios(send, token).await?;
        run_error_scenarios(send, token).await?;

        Ok(())
    })
}

fn assert_json_matches(actual: &Value, expected_path: &Utf8Path) -> Result<(), eyre::Report> {
    let expected = load_json_fixture(expected_path)?;
    eyre::ensure!(
        actual == &expected,
        "fixture mismatch for {}\nactual: {actual}\nexpected: {expected}",
        expected_path.as_str()
    );
    Ok(())
}

fn load_json_fixture(path: &Utf8Path) -> Result<Value, eyre::Report> {
    let parent = path.parent().ok_or_else(|| {
        eyre::eyre!(
            "fixture path {} should include a parent directory",
            path.as_str()
        )
    })?;
    let file_name = path
        .file_name()
        .ok_or_else(|| eyre::eyre!("fixture path {} should include a file name", path.as_str()))?;
    let fixture_dir = Dir::open_ambient_dir(parent, ambient_authority())
        .map_err(|err| eyre::eyre!("failed to open fixture directory {parent}: {err}"))?;
    let fixture = fixture_dir
        .read_to_string(file_name)
        .map_err(|err| eyre::eyre!("failed to read fixture {}: {err}", path.as_str()))?;
    serde_json::from_str(&fixture)
        .map_err(|err| eyre::eyre!("failed to parse fixture {}: {err}", path.as_str()))
}

fn normalize_task_success_response(body: &mut Value) -> Result<(), eyre::Report> {
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

fn normalize_shared_error_response(body: &mut Value) -> Result<(), eyre::Report> {
    replace_string_at_path(body, &["traceId"], Replacement::TraceId)
}

fn replace_string_at_path(
    body: &mut Value,
    path: &[&str],
    replacement: Replacement,
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
    let Value::String(target) = value else {
        return Err(eyre::eyre!(
            "expected JSON string at path leaf `{leaf}` for replacement `{}`, found {}",
            replacement.as_str(),
            serde_json::to_string(value).unwrap_or_else(|_| "<opaque>".to_owned())
        ));
    };
    replacement.as_str().clone_into(target);
    Ok(())
}
