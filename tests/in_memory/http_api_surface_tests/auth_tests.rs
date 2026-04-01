//! Authentication tests for the in-memory HTTP API surface.

use super::super::helpers::runtime;
use super::support::{assert_rejects_response, build_bundle, reject_conversations_request};
use crate::http_api_test_helpers::{required_field, required_str_field};
use actix_web::{App, web};
use corbusier::http_api::api_routes;
use rstest::rstest;
use serde_json::Value;
use std::io;
use tokio::runtime::Runtime;

#[derive(Clone, Copy)]
enum AuthRejectionCase {
    Missing,
    Invalid,
    UnsupportedTenantKind,
    MalformedUuids,
    Expired,
}

async fn run_auth_rejection_case(auth_case: AuthRejectionCase) -> Result<Value, eyre::Report> {
    let bundle = build_bundle().await?;
    let token = match auth_case {
        AuthRejectionCase::Missing => None,
        AuthRejectionCase::Invalid => Some("not-a-jwt".to_owned()),
        AuthRejectionCase::UnsupportedTenantKind => {
            Some(bundle.auth.token_with_tenant_kind("service")?)
        }
        AuthRejectionCase::MalformedUuids => Some(bundle.auth.token_with_invalid_uuids()?),
        AuthRejectionCase::Expired => Some(bundle.auth.expired_token()?),
    };
    let app = actix_web::test::init_service(
        App::new()
            .app_data(web::Data::new(bundle.state))
            .configure(api_routes),
    )
    .await;

    let request = token
        .as_deref()
        .map_or_else(
            || actix_web::test::TestRequest::post().uri("/api/v1/conversations"),
            reject_conversations_request,
        )
        .to_request();

    Ok(assert_rejects_response(actix_web::test::call_service(&app, request).await).await)
}

#[rstest]
#[case::missing(AuthRejectionCase::Missing, None)]
#[case::invalid(AuthRejectionCase::Invalid, None)]
#[case::unsupported_tenant_kind(
    AuthRejectionCase::UnsupportedTenantKind,
    Some("unsupported tenant kind")
)]
#[case::malformed_uuids(AuthRejectionCase::MalformedUuids, None)]
#[case::expired(AuthRejectionCase::Expired, None)]
fn rejects_authentication_failures(
    runtime: io::Result<Runtime>,
    #[case] auth_case: AuthRejectionCase,
    #[case] expected_message: Option<&str>,
) -> Result<(), eyre::Report> {
    let rt = runtime?;
    rt.block_on(async {
        let body = run_auth_rejection_case(auth_case).await?;
        if let Some(expected_substr) = expected_message {
            eyre::ensure!(
                required_str_field(required_field(&body, "error"), "message")
                    .contains(expected_substr),
                "error message should mention {expected_substr}"
            );
        }
        Ok(())
    })
}
