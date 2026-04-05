//! Tool route tests for the in-memory HTTP API surface.

use super::super::helpers::runtime;
use super::support::{assert_v1_metadata, build_bundle, with_bearer};
use crate::http_api_test_helpers::required_field;
use actix_web::{App, web};
use corbusier::http_api::api_routes;
use rstest::rstest;
use serde_json::{Value, json};
use std::io;
use tokio::runtime::Runtime;

#[rstest]
fn tool_routes_list_and_call_catalogued_tools(
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

        let list_request = with_bearer(
            actix_web::test::TestRequest::get().uri("/api/v1/tools"),
            &token,
        )
        .to_request();
        let list_response = actix_web::test::call_service(&app, list_request).await;
        eyre::ensure!(
            list_response.status().as_u16() == 200,
            "expected list response status 200, got {}",
            list_response.status().as_u16()
        );
        let list_body: Value = actix_web::test::read_body_json(list_response).await;
        assert_v1_metadata(&list_body);
        eyre::ensure!(
            required_field(required_field(&list_body, "data"), "tools")
                .as_array()
                .map(Vec::len)
                == Some(1),
            "expected exactly one tool in catalog"
        );

        let call_request = with_bearer(
            actix_web::test::TestRequest::post()
                .uri("/api/v1/tools/calls")
                .set_json(json!({
                    "tool_name": "read_file",
                    "parameters": { "path": "/tmp/example.txt" }
                })),
            &token,
        )
        .to_request();
        let call_response = actix_web::test::call_service(&app, call_request).await;
        eyre::ensure!(
            call_response.status().as_u16() == 200,
            "expected tool call response status 200, got {}",
            call_response.status().as_u16()
        );
        let call_body: Value = actix_web::test::read_body_json(call_response).await;
        assert_v1_metadata(&call_body);
        eyre::ensure!(
            *required_field(
                required_field(
                    required_field(required_field(&call_body, "data"), "outcome"),
                    "Success"
                ),
                "content"
            ) == json!({ "content": "hello from tool" }),
            "expected tool call content payload to match fixture result"
        );
        Ok(())
    })
}
