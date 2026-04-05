//! Tool HTTP routes for listing and invoking catalogued tools.
//!
//! This module exposes `GET /api/v1/tools`, which returns the authenticated
//! tenant's available tool catalog and metadata, and `POST /api/v1/tools/calls`,
//! which accepts a tool name plus JSON parameters and executes that tool
//! asynchronously through the tool-routing service. Clients should use the list
//! endpoint to discover callable tools and their descriptors before issuing a
//! call request; both endpoints require authentication, and the call endpoint
//! may trigger side effects depending on the selected tool.

use super::super::{
    auth::AuthenticatedRequestContext, error::ApiError, response::json_success, state::ApiState,
};
use actix_web::{HttpResponse, http::StatusCode, web};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tool_registry::domain::{
    CatalogEntry, ToolCallOutcome, ToolCallRequest, ToolCallResult,
};

#[derive(Debug, Deserialize)]
struct ToolCallBody {
    tool_name: String,
    parameters: Value,
}

#[derive(Debug, Serialize)]
struct ToolCatalogResponse {
    tools: Vec<CatalogEntry>,
}

#[derive(Debug, Serialize)]
struct ToolCallResponse {
    call_id: String,
    tool_name: String,
    server_id: String,
    outcome: ToolCallOutcome,
    duration_ms: u128,
    completed_at: chrono::DateTime<chrono::Utc>,
}

/// Registers the tool routes under `/api/v1`.
pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/tools").route(web::get().to(list_tools)))
        .service(web::resource("/tools/calls").route(web::post().to(call_tool)));
}

async fn list_tools(state: web::Data<ApiState>, auth: AuthenticatedRequestContext) -> HttpResponse {
    let request_id = auth.request_id();
    match state.tools.list_tools(auth.context()).await {
        Ok(tools) => json_success(
            &*state.clock,
            StatusCode::OK,
            ToolCatalogResponse { tools },
            request_id,
        ),
        Err(err) => ApiError::from(err).into_response(&*state.clock, request_id),
    }
}

async fn call_tool(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
    body: web::Json<ToolCallBody>,
) -> HttpResponse {
    let request_id = auth.request_id();
    let payload = body.into_inner();
    let request = ToolCallRequest::new(payload.tool_name, payload.parameters, &*state.clock);
    match state.tools.call_tool(auth.context(), &request).await {
        Ok(result) => json_success(
            &*state.clock,
            StatusCode::OK,
            map_tool_call_result(&result),
            request_id,
        ),
        Err(err) => ApiError::from(err).into_response(&*state.clock, request_id),
    }
}

fn map_tool_call_result(result: &ToolCallResult) -> ToolCallResponse {
    ToolCallResponse {
        call_id: result.call_id().to_string(),
        tool_name: result.tool_name().to_owned(),
        server_id: result.server_id().to_string(),
        outcome: result.outcome().clone(),
        duration_ms: result.duration().as_millis(),
        completed_at: result.completed_at(),
    }
}
