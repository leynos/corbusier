//! Tool HTTP routes.

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
    match state.tools.list_tools(auth.context()).await {
        Ok(tools) => json_success(
            StatusCode::OK,
            ToolCatalogResponse { tools },
            auth.request_id(),
        ),
        Err(err) => ApiError::from(err).into_response(auth.request_id()),
    }
}

async fn call_tool(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
    body: web::Json<ToolCallBody>,
) -> HttpResponse {
    let payload = body.into_inner();
    let request = ToolCallRequest::new(payload.tool_name, payload.parameters, &*state.clock);
    match state.tools.call_tool(auth.context(), &request).await {
        Ok(result) => json_success(
            StatusCode::OK,
            map_tool_call_result(&result),
            auth.request_id(),
        ),
        Err(err) => ApiError::from(err).into_response(auth.request_id()),
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
