//! `PostgreSQL` integration tests for issue-to-task creation and lookup.

use corbusier::context::RequestContext;
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
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, other_tenant_ctx, postgres_cluster,
    test_request_context,
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
    test_request_context: RequestContext,
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;
    let ctx = test_request_context;

    let request = CreateTaskFromIssueRequest::new(
        "github",
        "corbusier/core",
        321,
        "Persist task origin metadata",
    )
    .with_description("Verify task creation and retrieval in PostgreSQL");
    let created = service
        .create_from_issue(&ctx, request)
        .await
        .expect("task creation should succeed");

    let issue_ref =
        IssueRef::from_parts("github", "corbusier/core", 321).expect("valid issue reference");
    let found = service
        .find_by_issue_ref(&ctx, &issue_ref)
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
    test_request_context: RequestContext,
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;
    let ctx = test_request_context;

    service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new("gitlab", "corbusier/core", 17, "First task"),
        )
        .await
        .expect("first task creation should succeed");

    let duplicate_result = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new("gitlab", "corbusier/core", 17, "Duplicate task"),
        )
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
    test_request_context: RequestContext,
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;
    let ctx = test_request_context;

    let issue_ref =
        IssueRef::from_parts("github", "corbusier/core", 10001).expect("valid issue reference");
    let found = service
        .find_by_issue_ref(&ctx, &issue_ref)
        .await
        .expect("lookup should succeed");

    assert!(found.is_none());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_create_rejects_issue_number_beyond_supported_range(
    test_request_context: RequestContext,
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;
    let ctx = test_request_context;
    let too_large_issue_number = (i64::MAX as u64) + 1;

    let result = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new(
                "github",
                "corbusier/core",
                too_large_issue_number,
                "Out-of-range issue number",
            ),
        )
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
    test_request_context: RequestContext,
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;
    let repository = &task_context.repository;
    let ctx = test_request_context;

    let created = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new(
                "github",
                "corbusier/core",
                42,
                "find_by_id round trip",
            ),
        )
        .await
        .expect("task creation should succeed");

    let fetched = repository
        .find_by_id(&ctx, created.id())
        .await
        .expect("repository lookup should succeed")
        .expect("task should exist in repository");

    assert_eq!(fetched.id(), created.id());
    assert_eq!(fetched.origin(), created.origin());
    assert_eq!(fetched.state(), created.state());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_duplicate_issue_reference_is_scoped_per_tenant(
    test_request_context: RequestContext,
    #[future] context: Result<TaskTestContext, BoxError>,
) -> Result<(), BoxError> {
    let task_context = context.await?;
    let service = &task_context.service;
    let tenant_a = test_request_context;
    let tenant_b = other_tenant_ctx(&tenant_a);

    let task_a = service
        .create_from_issue(
            &tenant_a,
            CreateTaskFromIssueRequest::new(
                "github",
                "corbusier/core",
                4242,
                "Tenant A issue mapping",
            ),
        )
        .await
        .expect("tenant A task creation should succeed");
    let task_b = service
        .create_from_issue(
            &tenant_b,
            CreateTaskFromIssueRequest::new(
                "github",
                "corbusier/core",
                4242,
                "Tenant B issue mapping",
            ),
        )
        .await
        .expect("tenant B task creation should succeed");

    assert_ne!(task_a.id(), task_b.id(), "tenants must get distinct tasks");

    let issue_ref =
        IssueRef::from_parts("github", "corbusier/core", 4242).expect("valid issue reference");
    let found_a = service
        .find_by_issue_ref(&tenant_a, &issue_ref)
        .await
        .expect("tenant A lookup should succeed")
        .expect("tenant A task should exist");
    let found_b = service
        .find_by_issue_ref(&tenant_b, &issue_ref)
        .await
        .expect("tenant B lookup should succeed")
        .expect("tenant B task should exist");

    assert_eq!(found_a.id(), task_a.id());
    assert_eq!(found_b.id(), task_b.id());
    Ok(())
}
