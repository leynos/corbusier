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
        assert_eq!(list_response.status().as_u16(), 200);
        let list_body: Value = actix_web::test::read_body_json(list_response).await;
        assert_v1_metadata(&list_body);
        assert_eq!(
            required_field(required_field(&list_body, "data"), "tools")
                .as_array()
                .map(Vec::len),
            Some(1)
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
        assert_eq!(call_response.status().as_u16(), 200);
        let call_body: Value = actix_web::test::read_body_json(call_response).await;
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
