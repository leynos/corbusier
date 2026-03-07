//! Tests for task state machine transitions.

use super::{TestService, service};
use crate::in_memory::helpers::ctx;
use corbusier::context::RequestContext;
use corbusier::task::{
    domain::{ParseTaskStateError, TaskDomainError, TaskState},
    services::{CreateTaskFromIssueRequest, TaskLifecycleError, TransitionTaskRequest},
};
use rstest::rstest;

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn transition_task_from_draft_to_in_progress(service: TestService, ctx: RequestContext) {
    let task = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new(
                "github",
                "corbusier/core",
                306,
                "Transition happy path",
            ),
        )
        .await
        .expect("task creation should succeed");
    let transitioned = service
        .transition_task(&ctx, TransitionTaskRequest::new(task.id(), "in_progress"))
        .await
        .expect("transition should succeed");
    assert_eq!(transitioned.state(), TaskState::InProgress);
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn transition_rejects_invalid_state_change(service: TestService, ctx: RequestContext) {
    let task = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new(
                "github",
                "corbusier/core",
                307,
                "Transition invalid path",
            ),
        )
        .await
        .expect("task creation should succeed");
    let result = service
        .transition_task(&ctx, TransitionTaskRequest::new(task.id(), "done"))
        .await;
    assert!(matches!(
        result,
        Err(TaskLifecycleError::Domain(
            TaskDomainError::InvalidStateTransition {
                from: TaskState::Draft,
                to: TaskState::Done,
                ..
            }
        ))
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn transition_rejects_unknown_state_string(service: TestService, ctx: RequestContext) {
    let task = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new(
                "github",
                "corbusier/core",
                308,
                "Transition parse failure",
            ),
        )
        .await
        .expect("task creation should succeed");
    let result = service
        .transition_task(
            &ctx,
            TransitionTaskRequest::new(task.id(), "nonexistent_state"),
        )
        .await;
    assert!(matches!(
        result,
        Err(TaskLifecycleError::InvalidState(ParseTaskStateError(_)))
    ));
}
