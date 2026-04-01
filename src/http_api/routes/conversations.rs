//! Registers the `/api/v1/conversations` endpoints for creating
//! conversations, reading conversation history, and appending messages, which
//! together manage conversation resources and return conversation data.
//! [`routes`] is the public entrypoint used to mount these handlers on the
//! API router.

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
) -> HttpResponse {
    let request_id = auth.request_id();
    match state
        .conversations
        .create_conversation(auth.context())
        .await
    {
        Ok(conversation) => json_success(
            StatusCode::CREATED,
            ConversationResponse { conversation },
            request_id,
        ),
        Err(err) => ApiError::from(err).into_response(request_id),
    }
}

async fn get_history(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
    path: web::Path<ConversationPath>,
) -> HttpResponse {
    let request_id = auth.request_id();
    let conversation_id = match parse_conversation_id(&path.conversation_id) {
        Ok(id) => id,
        Err(err) => return err.into_response(request_id),
    };
    match state
        .conversations
        .history(auth.context(), conversation_id)
        .await
    {
        Ok(messages) => json_success(
            StatusCode::OK,
            ConversationHistoryResponse {
                conversation_id,
                messages,
            },
            request_id,
        ),
        Err(err) => ApiError::from(err).into_response(request_id),
    }
}

async fn append_message(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
    path: web::Path<ConversationPath>,
    body: web::Json<AppendMessageBody>,
) -> HttpResponse {
    let request_id = auth.request_id();
    let conversation_id = match parse_conversation_id(&path.conversation_id) {
        Ok(id) => id,
        Err(err) => return err.into_response(request_id),
    };
    let payload = body.into_inner();
    match state
        .conversations
        .append_message(
            auth.context(),
            AppendMessageRequest::new(conversation_id, payload.role, payload.content),
        )
        .await
    {
        Ok(message) => json_success(StatusCode::CREATED, MessageResponse { message }, request_id),
        Err(err) => ApiError::from(err).into_response(request_id),
    }
}

fn parse_conversation_id(raw: &str) -> Result<ConversationId, ApiError> {
    Uuid::parse_str(raw)
        .map(ConversationId::from_uuid)
        .map_err(|_| ApiError::bad_request("invalid_conversation_id", "invalid conversation id"))
}
