//! `PostgreSQL` integration tests for issue-to-task creation and lookup.

use corbusier::task::{
    adapters::postgres::{PostgresTaskRepository, TaskPgPool},
    domain::{IssueRef, TaskDomainError},
    ports::{TaskRepository, TaskRepositoryError},
    services::{CreateTaskFromIssueRequest, TaskLifecycleError, TaskLifecycleService},
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

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_create_and_lookup_by_issue_reference(
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;

    let request = CreateTaskFromIssueRequest::new(
        "github",
        "corbusier/core",
        321,
        "Persist task origin metadata",
    )
    .with_description("Verify task creation and retrieval in PostgreSQL");
    let created = service
        .create_from_issue(request)
        .await
        .expect("task creation should succeed");

    let issue_ref =
        IssueRef::from_parts("github", "corbusier/core", 321).expect("valid issue reference");
    let found = service
        .find_by_issue_ref(&issue_ref)
        .await
        .expect("lookup should succeed");

    let fetched = found.expect("task should be found by issue reference");
    assert_eq!(fetched.id(), created.id());
    assert_eq!(fetched.origin(), created.origin());
    assert_eq!(fetched.state(), created.state());
    assert_eq!(
        fetched.created_at().timestamp_micros(),
        created.created_at().timestamp_micros()
    );
    assert_eq!(
        fetched.updated_at().timestamp_micros(),
        created.updated_at().timestamp_micros()
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_duplicate_issue_reference_is_rejected(
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;

    service
        .create_from_issue(CreateTaskFromIssueRequest::new(
            "gitlab",
            "corbusier/core",
            17,
            "First task",
        ))
        .await
        .expect("first task creation should succeed");

    let duplicate_result = service
        .create_from_issue(CreateTaskFromIssueRequest::new(
            "gitlab",
            "corbusier/core",
            17,
            "Duplicate task",
        ))
        .await;

    assert!(matches!(
        duplicate_result,
        Err(TaskLifecycleError::Repository(
            TaskRepositoryError::DuplicateIssueOrigin(_)
        ))
    ));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_lookup_returns_none_for_missing_issue_reference(
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;

    let issue_ref =
        IssueRef::from_parts("github", "corbusier/core", 10001).expect("valid issue reference");
    let found = service
        .find_by_issue_ref(&issue_ref)
        .await
        .expect("lookup should succeed");

    assert!(found.is_none());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_create_rejects_issue_number_beyond_supported_range(
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;
    let too_large_issue_number = (i64::MAX as u64) + 1;

    let result = service
        .create_from_issue(CreateTaskFromIssueRequest::new(
            "github",
            "corbusier/core",
            too_large_issue_number,
            "Out-of-range issue number",
        ))
        .await;

    assert!(
        matches!(
            result,
            Err(TaskLifecycleError::Domain(TaskDomainError::InvalidIssueNumber(issue_number)))
                if issue_number == too_large_issue_number
        ),
        "expected InvalidIssueNumber domain error"
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_repository_find_by_id_round_trips_created_task(
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;
    let repository = &task_context.repository;

    let created = service
        .create_from_issue(CreateTaskFromIssueRequest::new(
            "github",
            "corbusier/core",
            42,
            "find_by_id round trip",
        ))
        .await
        .expect("task creation should succeed");

    let fetched = repository
        .find_by_id(created.id())
        .await
        .expect("repository lookup should succeed")
        .expect("task should exist in repository");

    assert_eq!(fetched.id(), created.id());
    assert_eq!(fetched.origin(), created.origin());
    assert_eq!(fetched.state(), created.state());
    Ok(())
}
