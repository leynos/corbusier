//! Authentication tests for the in-memory HTTP API surface.

use super::super::helpers::runtime;
use super::support::{
    assert_rejects_response, assert_v1_metadata, build_bundle, reject_conversations_request,
    with_bearer,
};
use crate::http_api_test_helpers::{required_field, required_str_field};
use actix_web::{App, web};
use corbusier::http_api::api_routes;
use rstest::rstest;
use serde_json::Value;
use std::io;
use tokio::runtime::Runtime;

#[rstest]
fn rejects_missing_and_invalid_bearer_tokens(runtime: io::Result<Runtime>) {
    let rt = runtime.unwrap_or_else(|err| panic!("runtime should be available: {err}"));
    rt.block_on(async {
        let bundle = build_bundle()
            .await
            .unwrap_or_else(|err| panic!("bundle setup should succeed: {err}"));
        let app = actix_web::test::init_service(
            App::new()
                .app_data(web::Data::new(bundle.state))
                .configure(api_routes),
        )
        .await;

        let missing_request = actix_web::test::TestRequest::post()
            .uri("/api/v1/conversations")
            .to_request();
        let missing_response = actix_web::test::call_service(&app, missing_request).await;
        assert_eq!(missing_response.status().as_u16(), 401);
        let missing_body: Value = actix_web::test::read_body_json(missing_response).await;
        assert_v1_metadata(&missing_body);

        let invalid_request = with_bearer(
            actix_web::test::TestRequest::post().uri("/api/v1/conversations"),
            "not-a-jwt",
        )
        .to_request();
        let invalid_response = actix_web::test::call_service(&app, invalid_request).await;
        assert_eq!(invalid_response.status().as_u16(), 401);
        let invalid_body: Value = actix_web::test::read_body_json(invalid_response).await;
        assert_v1_metadata(&invalid_body);
    });
}

#[rstest]
fn rejects_unsupported_tenant_kind(runtime: io::Result<Runtime>) {
    let rt = runtime.unwrap_or_else(|err| panic!("runtime should be available: {err}"));
    rt.block_on(async {
        let bundle = build_bundle()
            .await
            .unwrap_or_else(|err| panic!("bundle setup should succeed: {err}"));
        let token = bundle
            .auth
            .token_with_tenant_kind("service")
            .unwrap_or_else(|err| panic!("token encoding should succeed: {err}"));
        let app = actix_web::test::init_service(
            App::new()
                .app_data(web::Data::new(bundle.state))
                .configure(api_routes),
        )
        .await;

        let request = reject_conversations_request(&token).to_request();
        let body =
            assert_rejects_response(actix_web::test::call_service(&app, request).await).await;
        assert!(
            required_str_field(required_field(&body, "error"), "message")
                .contains("unsupported tenant kind"),
            "error message should mention unsupported tenant kind"
        );
    });
}

#[rstest]
fn rejects_malformed_uuid_claims(runtime: io::Result<Runtime>) {
    let rt = runtime.unwrap_or_else(|err| panic!("runtime should be available: {err}"));
    rt.block_on(async {
        let bundle = build_bundle()
            .await
            .unwrap_or_else(|err| panic!("bundle setup should succeed: {err}"));
        let token = bundle
            .auth
            .token_with_invalid_uuids()
            .unwrap_or_else(|err| panic!("token encoding should succeed: {err}"));
        let app = actix_web::test::init_service(
            App::new()
                .app_data(web::Data::new(bundle.state))
                .configure(api_routes),
        )
        .await;

        let request = reject_conversations_request(&token).to_request();
        assert_rejects_response(actix_web::test::call_service(&app, request).await).await;
    });
}

#[rstest]
fn rejects_expired_tokens(runtime: io::Result<Runtime>) {
    let rt = runtime.unwrap_or_else(|err| panic!("runtime should be available: {err}"));
    rt.block_on(async {
        let bundle = build_bundle()
            .await
            .unwrap_or_else(|err| panic!("bundle setup should succeed: {err}"));
        let token = bundle
            .auth
            .expired_token()
            .unwrap_or_else(|err| panic!("token encoding should succeed: {err}"));
        let app = actix_web::test::init_service(
            App::new()
                .app_data(web::Data::new(bundle.state))
                .configure(api_routes),
        )
        .await;

        let request = reject_conversations_request(&token).to_request();
        assert_rejects_response(actix_web::test::call_service(&app, request).await).await;
    });
}
