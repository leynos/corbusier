//! Conversation route tests for the in-memory HTTP API surface.

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
fn authenticated_conversation_flow(runtime: io::Result<Runtime>) -> Result<(), eyre::Report> {
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
            actix_web::test::TestRequest::post().uri("/api/v1/conversations"),
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
        let conversation_id = required_str_field(
            required_field(required_field(&create_body, "data"), "conversation"),
            "id",
        )
        .to_owned();

        let append_request = with_bearer(
            actix_web::test::TestRequest::post()
                .uri(&format!("/api/v1/conversations/{conversation_id}/messages"))
                .set_json(json!({
                    "role": "user",
                    "content": [{ "type": "text", "text": "Hello over HTTP" }]
                })),
            &token,
        )
        .to_request();
        let append_response = actix_web::test::call_service(&app, append_request).await;
        eyre::ensure!(
            append_response.status().as_u16() == 201,
            "expected append response status 201, got {}",
            append_response.status().as_u16()
        );
        let append_body: Value = actix_web::test::read_body_json(append_response).await;
        assert_v1_metadata(&append_body);

        let history_request = with_bearer(
            actix_web::test::TestRequest::get()
                .uri(&format!("/api/v1/conversations/{conversation_id}/history")),
            &token,
        )
        .to_request();
        let history_response = actix_web::test::call_service(&app, history_request).await;
        eyre::ensure!(
            history_response.status().as_u16() == 200,
            "expected history response status 200, got {}",
            history_response.status().as_u16()
        );
        let history_body: Value = actix_web::test::read_body_json(history_response).await;
        assert_v1_metadata(&history_body);
        eyre::ensure!(
            required_field(required_field(&history_body, "data"), "messages")
                .as_array()
                .map(Vec::len)
                == Some(1),
            "expected exactly one message in history"
        );

        Ok(())
    })
}
