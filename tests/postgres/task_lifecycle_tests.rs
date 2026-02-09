//! `PostgreSQL` integration tests for issue-to-task creation and lookup.

use corbusier::task::{
    adapters::postgres::{PostgresTaskRepository, TaskPgPool},
    domain::IssueRef,
    ports::TaskRepositoryError,
    services::{CreateTaskFromIssueRequest, TaskLifecycleError, TaskLifecycleService},
};
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use mockable::DefaultClock;
use rstest::rstest;
use std::sync::Arc;
use uuid::Uuid;

use crate::postgres::cluster::TemporaryDatabase;
use crate::postgres::helpers::{
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, postgres_cluster,
};

struct TaskTestContext {
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
    let service = TaskLifecycleService::new(repository, Arc::new(DefaultClock));
    Ok(TaskTestContext {
        service,
        _temp_db: db,
    })
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_create_and_lookup_by_issue_reference(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) {
    let cluster = postgres_cluster.expect("postgres cluster should start");
    ensure_template(cluster)
        .await
        .expect("template database should be prepared");
    let context = setup_task_context(cluster)
        .await
        .expect("task test context should be created");
    let service = &context.service;

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
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_duplicate_issue_reference_is_rejected(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) {
    let cluster = postgres_cluster.expect("postgres cluster should start");
    ensure_template(cluster)
        .await
        .expect("template database should be prepared");
    let context = setup_task_context(cluster)
        .await
        .expect("task test context should be created");
    let service = &context.service;

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
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_lookup_returns_none_for_missing_issue_reference(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) {
    let cluster = postgres_cluster.expect("postgres cluster should start");
    ensure_template(cluster)
        .await
        .expect("template database should be prepared");
    let context = setup_task_context(cluster)
        .await
        .expect("task test context should be created");
    let service = &context.service;

    let issue_ref =
        IssueRef::from_parts("github", "corbusier/core", 10001).expect("valid issue reference");
    let found = service
        .find_by_issue_ref(&issue_ref)
        .await
        .expect("lookup should succeed");

    assert!(found.is_none());
}
