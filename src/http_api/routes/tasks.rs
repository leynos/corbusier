//! Registers the authenticated task routes mounted under `/api/v1`, including
//! `POST /tasks`, `GET /tasks/{id}`, task state transitions, and branch or
//! pull-request association operations handled through
//! [`AuthenticatedRequestContext`]. [`routes`] is the public entrypoint that
//! wires these task-management handlers into the HTTP router.

use super::super::{
    auth::AuthenticatedRequestContext, error::ApiError, response::json_success, state::ApiState,
};
use actix_v2a::{extract_idempotency_key, map_idempotency_key_error};
use actix_web::{FromRequest, HttpRequest, HttpResponse, dev::Payload, http::StatusCode, web};
use futures::future::{Ready, ready};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::task::{
    domain::{BranchRef, PullRequestRef, Task, TaskId, TaskOrigin, TaskState},
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
struct TaskDto {
    id: TaskId,
    origin: TaskOrigin,
    branch_ref: Option<BranchRef>,
    pull_request_ref: Option<PullRequestRef>,
    state: TaskState,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<Task> for TaskDto {
    fn from(task: Task) -> Self {
        Self {
            id: task.id(),
            origin: task.origin().clone(),
            branch_ref: task.branch_ref().cloned(),
            pull_request_ref: task.pull_request_ref().cloned(),
            state: task.state(),
            created_at: task.created_at(),
            updated_at: task.updated_at(),
        }
    }
}

#[derive(Debug, Serialize)]
struct TaskResponse {
    task: TaskDto,
}

#[derive(Debug, Clone)]
struct TaskMutationContext {
    auth: AuthenticatedRequestContext,
}

impl TaskMutationContext {
    fn request_id(&self) -> String {
        self.auth.request_id()
    }

    const fn context(&self) -> &crate::context::RequestContext {
        self.auth.context()
    }
}

impl FromRequest for TaskMutationContext {
    type Error = ApiError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(request: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let auth = match AuthenticatedRequestContext::from_request(request, payload).into_inner() {
            Ok(auth) => auth,
            Err(err) => return ready(Err(err)),
        };

        ready(validate_idempotency_header(request).map(|()| Self { auth }))
    }
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
    auth: TaskMutationContext,
    body: web::Json<CreateTaskBody>,
) -> HttpResponse {
    let request_id = auth.request_id();
    let create_request = build_create_task_request(body.into_inner());
    match state
        .tasks
        .create_task(auth.context(), create_request)
        .await
    {
        Ok(task) => json_success(
            &*state.clock,
            StatusCode::CREATED,
            TaskResponse { task: task.into() },
            request_id,
        ),
        Err(err) => ApiError::from(err).into_response(&*state.clock, request_id),
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
        Err(err) => return err.into_response(&*state.clock, request_id),
    };
    match state.tasks.get_task(auth.context(), task_id).await {
        Ok(task) => json_success(
            &*state.clock,
            StatusCode::OK,
            TaskResponse { task: task.into() },
            request_id,
        ),
        Err(err) => ApiError::from(err).into_response(&*state.clock, request_id),
    }
}

async fn transition_task(
    state: web::Data<ApiState>,
    auth: TaskMutationContext,
    path: web::Path<TaskPath>,
    body: web::Json<TransitionTaskBody>,
) -> HttpResponse {
    let request_id = auth.request_id();
    let task_id = match parse_task_id(&path.task_id) {
        Ok(id) => id,
        Err(err) => return err.into_response(&*state.clock, request_id),
    };
    match state
        .tasks
        .transition_task(
            auth.context(),
            TransitionTaskRequest::new(task_id, body.into_inner().state),
        )
        .await
    {
        Ok(task) => json_success(
            &*state.clock,
            StatusCode::OK,
            TaskResponse { task: task.into() },
            request_id,
        ),
        Err(err) => ApiError::from(err).into_response(&*state.clock, request_id),
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
        Err(err) => return err.into_response(&*state.clock, request_id),
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
        Ok(task) => json_success(
            &*state.clock,
            StatusCode::OK,
            TaskResponse { task: task.into() },
            request_id,
        ),
        Err(err) => ApiError::from(err).into_response(&*state.clock, request_id),
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
        Err(err) => return err.into_response(&*state.clock, request_id),
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
        Ok(task) => json_success(
            &*state.clock,
            StatusCode::OK,
            TaskResponse { task: task.into() },
            request_id,
        ),
        Err(err) => ApiError::from(err).into_response(&*state.clock, request_id),
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
    let mut req = CreateTaskFromIssueRequest::new(provider, repository, issue_number, title);
    if let Some(value) = description {
        req = req.with_description(value);
    }
    if let Some(value) = labels {
        req = req.with_labels(value);
    }
    if let Some(value) = assignees {
        req = req.with_assignees(value);
    }
    if let Some(value) = milestone {
        req = req.with_milestone(value);
    }
    req
}

fn parse_task_id(raw: &str) -> Result<TaskId, ApiError> {
    Uuid::parse_str(raw)
        .map(TaskId::from_uuid)
        .map_err(|_| ApiError::bad_request("invalid_task_id", "invalid task id"))
}

fn validate_idempotency_header(request: &HttpRequest) -> Result<(), ApiError> {
    extract_idempotency_key(request.headers())
        .map(|_| ())
        .map_err(|error| {
            let shared_error = map_idempotency_key_error(&error);
            ApiError::bad_request("invalid_idempotency_key", shared_error.message())
        })
}
