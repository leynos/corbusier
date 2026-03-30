//! Conversation HTTP routes.

use super::super::{
    auth::AuthenticatedRequestContext, error::ApiError, response::json_success, state::ApiState,
};
use actix_web::{HttpResponse, http::StatusCode, web};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::message::domain::{ContentPart, Conversation, ConversationId, Message, Role};
use crate::message::services::AppendMessageRequest;

#[derive(Debug, Deserialize)]
struct ConversationPath {
    conversation_id: String,
}

#[derive(Debug, Deserialize)]
struct AppendMessageBody {
    role: Role,
    content: Vec<ContentPart>,
}

#[derive(Debug, Serialize)]
struct ConversationResponse {
    conversation: Conversation,
}

#[derive(Debug, Serialize)]
struct ConversationHistoryResponse {
    conversation_id: ConversationId,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct MessageResponse {
    message: Message,
}

/// Registers the conversation routes under `/api/v1`.
pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/conversations").route(web::post().to(create_conversation)))
        .service(
            web::resource("/conversations/{conversation_id}/history")
                .route(web::get().to(get_history)),
        )
        .service(
            web::resource("/conversations/{conversation_id}/messages")
                .route(web::post().to(append_message)),
        );
}

async fn create_conversation(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
) -> Result<HttpResponse, ApiError> {
    let conversation = state
        .conversations
        .create_conversation(auth.context())
        .await?;
    Ok(json_success(
        StatusCode::CREATED,
        ConversationResponse { conversation },
        auth.request_id(),
    ))
}

async fn get_history(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
    path: web::Path<ConversationPath>,
) -> Result<HttpResponse, ApiError> {
    let conversation_id = parse_conversation_id(&path.conversation_id)?;
    let messages = state
        .conversations
        .history(auth.context(), conversation_id)
        .await?;
    Ok(json_success(
        StatusCode::OK,
        ConversationHistoryResponse {
            conversation_id,
            messages,
        },
        auth.request_id(),
    ))
}

async fn append_message(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
    path: web::Path<ConversationPath>,
    body: web::Json<AppendMessageBody>,
) -> Result<HttpResponse, ApiError> {
    let conversation_id = parse_conversation_id(&path.conversation_id)?;
    let payload = body.into_inner();
    let message = state
        .conversations
        .append_message(
            auth.context(),
            AppendMessageRequest::new(conversation_id, payload.role, payload.content),
        )
        .await?;
    Ok(json_success(
        StatusCode::CREATED,
        MessageResponse { message },
        auth.request_id(),
    ))
}

fn parse_conversation_id(raw: &str) -> Result<ConversationId, ApiError> {
    Uuid::parse_str(raw)
        .map(ConversationId::from_uuid)
        .map_err(|_| ApiError::bad_request("invalid_conversation_id", "invalid conversation id"))
}
