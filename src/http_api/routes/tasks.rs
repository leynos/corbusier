//! Task HTTP routes.

use super::super::{
    auth::AuthenticatedRequestContext, error::ApiError, response::json_success, state::ApiState,
};
use actix_web::{HttpResponse, http::StatusCode, web};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::task::{
    domain::{Task, TaskId},
    services::{
        AssociateBranchRequest, AssociatePullRequestRequest, CreateTaskFromIssueRequest,
        TransitionTaskRequest,
    },
};

#[derive(Debug, Deserialize)]
struct TaskPath {
    task_id: String,
}

#[derive(Debug, Deserialize)]
struct CreateTaskBody {
    provider: String,
    repository: String,
    issue_number: u64,
    title: String,
    description: Option<String>,
    labels: Option<Vec<String>>,
    assignees: Option<Vec<String>>,
    milestone: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TransitionTaskBody {
    state: String,
}

#[derive(Debug, Deserialize)]
struct AssociateBranchBody {
    provider: String,
    repository: String,
    branch_name: String,
}

#[derive(Debug, Deserialize)]
struct AssociatePullRequestBody {
    provider: String,
    repository: String,
    pull_request_number: u64,
}

#[derive(Debug, Serialize)]
struct TaskResponse {
    task: Task,
}

/// Registers the task routes under `/api/v1`.
pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/tasks").route(web::post().to(create_task)))
        .service(web::resource("/tasks/{task_id}").route(web::get().to(get_task)))
        .service(web::resource("/tasks/{task_id}/state").route(web::put().to(transition_task)))
        .service(web::resource("/tasks/{task_id}/branch").route(web::put().to(associate_branch)))
        .service(
            web::resource("/tasks/{task_id}/pull-request")
                .route(web::put().to(associate_pull_request)),
        );
}

async fn create_task(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
    body: web::Json<CreateTaskBody>,
) -> Result<HttpResponse, ApiError> {
    let request = build_create_task_request(body.into_inner());
    let task = state.tasks.create_task(auth.context(), request).await?;
    Ok(json_success(
        StatusCode::CREATED,
        TaskResponse { task },
        auth.request_id(),
    ))
}

async fn get_task(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
    path: web::Path<TaskPath>,
) -> Result<HttpResponse, ApiError> {
    let task = state
        .tasks
        .get_task(auth.context(), parse_task_id(&path.task_id)?)
        .await?;
    Ok(json_success(
        StatusCode::OK,
        TaskResponse { task },
        auth.request_id(),
    ))
}

async fn transition_task(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
    path: web::Path<TaskPath>,
    body: web::Json<TransitionTaskBody>,
) -> Result<HttpResponse, ApiError> {
    let task_id = parse_task_id(&path.task_id)?;
    let task = state
        .tasks
        .transition_task(
            auth.context(),
            TransitionTaskRequest::new(task_id, body.into_inner().state),
        )
        .await?;
    Ok(json_success(
        StatusCode::OK,
        TaskResponse { task },
        auth.request_id(),
    ))
}

async fn associate_branch(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
    path: web::Path<TaskPath>,
    body: web::Json<AssociateBranchBody>,
) -> Result<HttpResponse, ApiError> {
    let task_id = parse_task_id(&path.task_id)?;
    let payload = body.into_inner();
    let task = state
        .tasks
        .associate_branch(
            auth.context(),
            AssociateBranchRequest::new(
                task_id,
                payload.provider,
                payload.repository,
                payload.branch_name,
            ),
        )
        .await?;
    Ok(json_success(
        StatusCode::OK,
        TaskResponse { task },
        auth.request_id(),
    ))
}

async fn associate_pull_request(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
    path: web::Path<TaskPath>,
    body: web::Json<AssociatePullRequestBody>,
) -> Result<HttpResponse, ApiError> {
    let task_id = parse_task_id(&path.task_id)?;
    let payload = body.into_inner();
    let task = state
        .tasks
        .associate_pull_request(
            auth.context(),
            AssociatePullRequestRequest::new(
                task_id,
                payload.provider,
                payload.repository,
                payload.pull_request_number,
            ),
        )
        .await?;
    Ok(json_success(
        StatusCode::OK,
        TaskResponse { task },
        auth.request_id(),
    ))
}

fn build_create_task_request(body: CreateTaskBody) -> CreateTaskFromIssueRequest {
    let base_request = CreateTaskFromIssueRequest::new(
        body.provider,
        body.repository,
        body.issue_number,
        body.title,
    );
    let with_description = match body.description {
        Some(description) => base_request.with_description(description),
        None => base_request,
    };
    let with_labels = match body.labels {
        Some(labels) => with_description.with_labels(labels),
        None => with_description,
    };
    let with_assignees = match body.assignees {
        Some(assignees) => with_labels.with_assignees(assignees),
        None => with_labels,
    };
    match body.milestone {
        Some(milestone) => with_assignees.with_milestone(milestone),
        None => with_assignees,
    }
}

fn parse_task_id(raw: &str) -> Result<TaskId, ApiError> {
    Uuid::parse_str(raw)
        .map(TaskId::from_uuid)
        .map_err(|_| ApiError::bad_request("invalid_task_id", "invalid task id"))
}
