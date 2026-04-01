//! Registers the authenticated task routes mounted under `/api/v1`, including
//! `POST /tasks`, `GET /tasks/{id}`, task state transitions, and branch or
//! pull-request association operations handled through
//! [`AuthenticatedRequestContext`]. [`routes`] is the public entrypoint that
//! wires these task-management handlers into the HTTP router.

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
) -> HttpResponse {
    let request_id = auth.request_id();
    let request = build_create_task_request(body.into_inner());
    match state.tasks.create_task(auth.context(), request).await {
        Ok(task) => json_success(StatusCode::CREATED, TaskResponse { task }, request_id),
        Err(err) => ApiError::from(err).into_response(request_id),
    }
}

async fn get_task(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
    path: web::Path<TaskPath>,
) -> HttpResponse {
    let request_id = auth.request_id();
    let task_id = match parse_task_id(&path.task_id) {
        Ok(id) => id,
        Err(err) => return err.into_response(request_id),
    };
    match state.tasks.get_task(auth.context(), task_id).await {
        Ok(task) => json_success(StatusCode::OK, TaskResponse { task }, request_id),
        Err(err) => ApiError::from(err).into_response(request_id),
    }
}

async fn transition_task(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
    path: web::Path<TaskPath>,
    body: web::Json<TransitionTaskBody>,
) -> HttpResponse {
    let request_id = auth.request_id();
    let task_id = match parse_task_id(&path.task_id) {
        Ok(id) => id,
        Err(err) => return err.into_response(request_id),
    };
    match state
        .tasks
        .transition_task(
            auth.context(),
            TransitionTaskRequest::new(task_id, body.into_inner().state),
        )
        .await
    {
        Ok(task) => json_success(StatusCode::OK, TaskResponse { task }, request_id),
        Err(err) => ApiError::from(err).into_response(request_id),
    }
}

async fn associate_branch(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
    path: web::Path<TaskPath>,
    body: web::Json<AssociateBranchBody>,
) -> HttpResponse {
    let request_id = auth.request_id();
    let task_id = match parse_task_id(&path.task_id) {
        Ok(id) => id,
        Err(err) => return err.into_response(request_id),
    };
    let payload = body.into_inner();
    match state
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
        .await
    {
        Ok(task) => json_success(StatusCode::OK, TaskResponse { task }, request_id),
        Err(err) => ApiError::from(err).into_response(request_id),
    }
}

async fn associate_pull_request(
    state: web::Data<ApiState>,
    auth: AuthenticatedRequestContext,
    path: web::Path<TaskPath>,
    body: web::Json<AssociatePullRequestBody>,
) -> HttpResponse {
    let request_id = auth.request_id();
    let task_id = match parse_task_id(&path.task_id) {
        Ok(id) => id,
        Err(err) => return err.into_response(request_id),
    };
    let payload = body.into_inner();
    match state
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
        .await
    {
        Ok(task) => json_success(StatusCode::OK, TaskResponse { task }, request_id),
        Err(err) => ApiError::from(err).into_response(request_id),
    }
}

fn build_create_task_request(body: CreateTaskBody) -> CreateTaskFromIssueRequest {
    let CreateTaskBody {
        provider,
        repository,
        issue_number,
        title,
        description,
        labels,
        assignees,
        milestone,
    } = body;
    let base_request = CreateTaskFromIssueRequest::new(provider, repository, issue_number, title);
    let mut req = description
        .into_iter()
        .fold(base_request, CreateTaskFromIssueRequest::with_description);
    req = labels
        .into_iter()
        .fold(req, CreateTaskFromIssueRequest::with_labels);
    req = assignees
        .into_iter()
        .fold(req, CreateTaskFromIssueRequest::with_assignees);
    req = milestone
        .into_iter()
        .fold(req, CreateTaskFromIssueRequest::with_milestone);
    req
}

fn parse_task_id(raw: &str) -> Result<TaskId, ApiError> {
    Uuid::parse_str(raw)
        .map(TaskId::from_uuid)
        .map_err(|_| ApiError::bad_request("invalid_task_id", "invalid task id"))
}
