//! `PostgreSQL` integration tests for branch and PR association.

use corbusier::task::{
    adapters::postgres::{PostgresTaskRepository, TaskPgPool},
    domain::{BranchRef, PullRequestRef, TaskId, TaskState},
    ports::{TaskRepository, TaskRepositoryError},
    services::{
        AssociateBranchRequest, AssociatePullRequestRequest, CreateTaskFromIssueRequest,
        TaskLifecycleError, TaskLifecycleService,
    },
};
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use std::sync::Arc;
use uuid::Uuid;

use crate::postgres::cluster::TemporaryDatabase;
use crate::postgres::helpers::{
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, postgres_cluster,
};

struct TaskTestContext {
    repository: Arc<PostgresTaskRepository>,
    service: TaskLifecycleService<PostgresTaskRepository, DefaultClock>,
    _temp_db: TemporaryDatabase,
}

async fn setup_task_context(cluster: PostgresCluster) -> Result<TaskTestContext, BoxError> {
    let db = cluster
        .temporary_database_from_template(&format!("task_{}", Uuid::new_v4()), TEMPLATE_DB)
        .await?;
    let url = db.url().to_owned();

    let manager = ConnectionManager::<PgConnection>::new(url);
    let pool: TaskPgPool = diesel::r2d2::Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|err| Box::new(err) as BoxError)?;
    let repository = Arc::new(PostgresTaskRepository::new(pool));
    let service = TaskLifecycleService::new(repository.clone(), Arc::new(DefaultClock));
    Ok(TaskTestContext {
        repository,
        service,
        _temp_db: db,
    })
}

#[fixture]
async fn context(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<TaskTestContext, BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    setup_task_context(cluster).await
}

async fn create_pg_task(
    service: &TaskLifecycleService<PostgresTaskRepository, DefaultClock>,
    issue_number: u64,
) -> Result<corbusier::task::domain::Task, BoxError> {
    Ok(service
        .create_from_issue(CreateTaskFromIssueRequest::new(
            "github",
            "corbusier/core",
            issue_number,
            format!("PG task for issue #{issue_number}"),
        ))
        .await?)
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_associate_branch_and_find_by_id(
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;

    let task = create_pg_task(service, 800).await?;
    let updated = service
        .associate_branch(AssociateBranchRequest::new(
            task.id(),
            "github",
            "corbusier/core",
            "feature/pg-branch",
        ))
        .await
        .expect("branch association should succeed");

    assert!(updated.branch_ref().is_some());

    let fetched = task_context
        .repository
        .find_by_id(task.id())
        .await?
        .expect("task should exist");
    assert!(fetched.branch_ref().is_some());
    assert_eq!(
        fetched
            .branch_ref()
            .expect("branch_ref should be set")
            .to_string(),
        "github:corbusier/core:feature/pg-branch"
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_associate_pr_and_verify_state(
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;

    let task = create_pg_task(service, 801).await?;
    let updated = service
        .associate_pull_request(AssociatePullRequestRequest::new(
            task.id(),
            "github",
            "corbusier/core",
            77,
        ))
        .await
        .expect("PR association should succeed");

    assert!(updated.pull_request_ref().is_some());
    assert_eq!(updated.state(), TaskState::InReview);

    let fetched = task_context
        .repository
        .find_by_id(task.id())
        .await?
        .expect("task should exist");
    assert!(fetched.pull_request_ref().is_some());
    assert_eq!(
        fetched
            .pull_request_ref()
            .expect("pull_request_ref should be set")
            .to_string(),
        "github:corbusier/core:77"
    );
    assert_eq!(fetched.state(), TaskState::InReview);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_find_by_branch_ref_returns_matching_tasks(
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;

    let task = create_pg_task(service, 802).await?;
    service
        .associate_branch(AssociateBranchRequest::new(
            task.id(),
            "github",
            "corbusier/core",
            "feature/find-test",
        ))
        .await
        .expect("branch association should succeed");

    let branch_ref = BranchRef::from_parts("github", "corbusier/core", "feature/find-test")
        .expect("valid branch ref");
    let found = service
        .find_by_branch_ref(&branch_ref)
        .await
        .expect("lookup should succeed");
    assert_eq!(found.len(), 1);
    assert_eq!(found.first().expect("at least one task").id(), task.id());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_find_by_pr_ref_returns_matching_tasks(
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;

    let task = create_pg_task(service, 803).await?;
    service
        .associate_pull_request(AssociatePullRequestRequest::new(
            task.id(),
            "github",
            "corbusier/core",
            88,
        ))
        .await
        .expect("PR association should succeed");

    let pr_ref = PullRequestRef::from_parts("github", "corbusier/core", 88).expect("valid PR ref");
    let found = service
        .find_by_pull_request_ref(&pr_ref)
        .await
        .expect("lookup should succeed");
    assert_eq!(found.len(), 1);
    assert_eq!(found.first().expect("at least one task").id(), task.id());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_update_on_nonexistent_task_returns_not_found(
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;
    let unknown_id = TaskId::new();

    let result = service
        .associate_branch(AssociateBranchRequest::new(
            unknown_id,
            "github",
            "corbusier/core",
            "main",
        ))
        .await;

    assert!(matches!(
        result,
        Err(TaskLifecycleError::Repository(
            TaskRepositoryError::NotFound(_)
        ))
    ));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_two_tasks_sharing_branch_both_returned(
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;

    let task1 = create_pg_task(service, 804).await?;
    let task2 = create_pg_task(service, 805).await?;

    service
        .associate_branch(AssociateBranchRequest::new(
            task1.id(),
            "github",
            "corbusier/core",
            "shared/pg-branch",
        ))
        .await
        .expect("first task branch association should succeed");
    service
        .associate_branch(AssociateBranchRequest::new(
            task2.id(),
            "github",
            "corbusier/core",
            "shared/pg-branch",
        ))
        .await
        .expect("second task branch association should succeed");

    let branch_ref = BranchRef::from_parts("github", "corbusier/core", "shared/pg-branch")
        .expect("valid branch ref");
    let found = service
        .find_by_branch_ref(&branch_ref)
        .await
        .expect("lookup should succeed");
    assert_eq!(found.len(), 2);
    let ids: Vec<_> = found
        .iter()
        .map(corbusier::task::domain::Task::id)
        .collect();
    assert!(ids.contains(&task1.id()));
    assert!(ids.contains(&task2.id()));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_two_tasks_sharing_pull_request_both_returned(
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;

    let task1 = create_pg_task(service, 806).await?;
    let task2 = create_pg_task(service, 807).await?;

    service
        .associate_pull_request(AssociatePullRequestRequest::new(
            task1.id(),
            "github",
            "corbusier/core",
            99,
        ))
        .await
        .expect("first task PR association should succeed");
    service
        .associate_pull_request(AssociatePullRequestRequest::new(
            task2.id(),
            "github",
            "corbusier/core",
            99,
        ))
        .await
        .expect("second task PR association should succeed");

    let pr_ref = PullRequestRef::from_parts("github", "corbusier/core", 99).expect("valid PR ref");
    let found = service
        .find_by_pull_request_ref(&pr_ref)
        .await
        .expect("lookup should succeed");
    assert_eq!(found.len(), 2);
    let ids: Vec<_> = found
        .iter()
        .map(corbusier::task::domain::Task::id)
        .collect();
    assert!(ids.contains(&task1.id()));
    assert!(ids.contains(&task2.id()));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_find_by_branch_ref_returns_empty_when_none_match(
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;

    let branch_ref = BranchRef::from_parts("github", "corbusier/core", "no-such/branch")
        .expect("valid branch ref");
    let found = service
        .find_by_branch_ref(&branch_ref)
        .await
        .expect("lookup should succeed");
    assert!(
        found.is_empty(),
        "expected empty result for unmatched branch ref"
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_find_by_pr_ref_returns_empty_when_none_match(
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;

    let pr_ref =
        PullRequestRef::from_parts("github", "corbusier/core", 9999).expect("valid PR ref");
    let found = service
        .find_by_pull_request_ref(&pr_ref)
        .await
        .expect("lookup should succeed");
    assert!(
        found.is_empty(),
        "expected empty result for unmatched PR ref"
    );
    Ok(())
}
