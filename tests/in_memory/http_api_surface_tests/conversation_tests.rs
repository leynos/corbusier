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

async fn create_conversation<F, Fut, B>(send: &F, token: &str) -> Result<String, eyre::Report>
where
    F: Fn(actix_web::test::TestRequest) -> Fut,
    Fut: std::future::Future<Output = actix_web::dev::ServiceResponse<B>>,
    B: actix_web::body::MessageBody,
{
    let response = send(with_bearer(
        actix_web::test::TestRequest::post().uri("/api/v1/conversations"),
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
    Ok(required_str_field(
        required_field(required_field(&body, "data"), "conversation"),
        "id",
    )
    .to_owned())
}

async fn append_message<F, Fut, B>(
    send: &F,
    token: &str,
    conversation_id: &str,
) -> Result<(), eyre::Report>
where
    F: Fn(actix_web::test::TestRequest) -> Fut,
    Fut: std::future::Future<Output = actix_web::dev::ServiceResponse<B>>,
    B: actix_web::body::MessageBody,
{
    let response = send(with_bearer(
        actix_web::test::TestRequest::post()
            .uri(&format!("/api/v1/conversations/{conversation_id}/messages"))
            .set_json(json!({
                "role": "user",
                "content": [{ "type": "text", "text": "Hello over HTTP" }]
            })),
        token,
    ))
    .await;
    eyre::ensure!(
        response.status().as_u16() == 201,
        "expected append response status 201, got {}",
        response.status().as_u16()
    );
    let body: Value = actix_web::test::read_body_json(response).await;
    assert_v1_metadata(&body);
    Ok(())
}

async fn assert_history_has_one_message<F, Fut, B>(
    send: &F,
    token: &str,
    conversation_id: &str,
) -> Result<(), eyre::Report>
where
    F: Fn(actix_web::test::TestRequest) -> Fut,
    Fut: std::future::Future<Output = actix_web::dev::ServiceResponse<B>>,
    B: actix_web::body::MessageBody,
{
    let response = send(with_bearer(
        actix_web::test::TestRequest::get()
            .uri(&format!("/api/v1/conversations/{conversation_id}/history")),
        token,
    ))
    .await;
    eyre::ensure!(
        response.status().as_u16() == 200,
        "expected history response status 200, got {}",
        response.status().as_u16()
    );
    let body: Value = actix_web::test::read_body_json(response).await;
    assert_v1_metadata(&body);
    eyre::ensure!(
        required_field(required_field(&body, "data"), "messages")
            .as_array()
            .map(Vec::len)
            == Some(1),
        "expected exactly one message in history"
    );
    Ok(())
}

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
        let call = |request: actix_web::test::TestRequest| {
            actix_web::test::call_service(&app, request.to_request())
        };

        let conversation_id = create_conversation(&call, &token).await?;
        append_message(&call, &token, &conversation_id).await?;
        assert_history_has_one_message(&call, &token, &conversation_id).await?;

        Ok(())
    })
}
