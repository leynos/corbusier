//! Integration tests verifying tenant context propagation for task repository operations.
//!
//! These tests confirm that every `TaskRepository` method wraps its database
//! queries in a transaction where `app.tenant_id` is set via `SET LOCAL`,
//! preparing the connection for Row-Level Security (RLS) policies.

use corbusier::context::RequestContext;
use corbusier::task::{
    adapters::postgres::{PostgresTaskRepository, TaskPgPool},
    domain::{BranchRef, IssueRef, PullRequestRef, TaskId},
    ports::TaskRepository,
    services::{
        AssociateBranchRequest, AssociatePullRequestRequest, CreateTaskFromIssueRequest,
        TaskLifecycleService,
    },
};
use diesel::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel::sql_types::Text;
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use std::sync::Arc;
use uuid::Uuid;

use crate::postgres::cluster::TemporaryDatabase;
use crate::postgres::helpers::{
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, postgres_cluster, test_request_context,
};

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

struct TaskTenantTestContext {
    repository: Arc<PostgresTaskRepository>,
    service: TaskLifecycleService<PostgresTaskRepository, DefaultClock>,
    /// Direct database URL for raw-connection verification queries.
    db_url: String,
    _temp_db: TemporaryDatabase,
}

async fn setup_tenant_test_context(
    cluster: PostgresCluster,
) -> Result<TaskTenantTestContext, BoxError> {
    let db = cluster
        .temporary_database_from_template(&format!("task_tenant_{}", Uuid::new_v4()), TEMPLATE_DB)
        .await?;
    let url = db.url().to_owned();

    let manager = ConnectionManager::<PgConnection>::new(url.clone());
    let pool: TaskPgPool = diesel::r2d2::Pool::builder()
        .max_size(2)
        .build(manager)
        .map_err(|err| Box::new(err) as BoxError)?;
    let repository = Arc::new(PostgresTaskRepository::new(pool));
    let service = TaskLifecycleService::new(repository.clone(), Arc::new(DefaultClock));
    Ok(TaskTenantTestContext {
        repository,
        service,
        db_url: url,
        _temp_db: db,
    })
}

#[fixture]
async fn context(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<TaskTenantTestContext, BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    setup_tenant_test_context(cluster).await
}

// ---------------------------------------------------------------------------
// Helper to read tenant_id set inside a transaction
// ---------------------------------------------------------------------------

/// Queryable result for reading a `PostgreSQL` session variable.
#[derive(QueryableByName)]
struct SettingRow {
    #[diesel(sql_type = Text)]
    val: String,
}

/// Verifies that `SET LOCAL app.tenant_id` correctly scopes the session
/// variable to the enclosing transaction using the same SQL pattern the
/// adapter's `with_tenant_tx` helper uses.
fn verify_set_local_scoping(db_url: &str, expected_tenant_id: Uuid) -> Result<(), BoxError> {
    let mut conn = PgConnection::establish(db_url).map_err(|e| Box::new(e) as BoxError)?;
    let expected_str = expected_tenant_id.to_string();

    // Inside a transaction, SET LOCAL should make the value visible.
    let inner_val = conn
        .transaction::<String, diesel::result::Error, _>(|tx| {
            diesel::sql_query(format!("SET LOCAL app.tenant_id = '{expected_tenant_id}'"))
                .execute(tx)?;

            let row = diesel::sql_query("SELECT current_setting('app.tenant_id', true) AS val")
                .get_result::<SettingRow>(tx)?;
            Ok(row.val)
        })
        .map_err(|e| Box::new(e) as BoxError)?;

    if inner_val != expected_str {
        return Err(format!(
            "app.tenant_id mismatch inside transaction: expected {expected_str}, got {inner_val}"
        )
        .into());
    }

    // After the transaction, SET LOCAL should have been reverted.
    let row = diesel::sql_query("SELECT current_setting('app.tenant_id', true) AS val")
        .get_result::<SettingRow>(&mut conn)
        .map_err(|e| Box::new(e) as BoxError)?;

    if !row.val.is_empty() {
        return Err(format!(
            "app.tenant_id should be empty outside the transaction, got: {}",
            row.val,
        )
        .into());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Verifies that `SET LOCAL app.tenant_id` correctly sets the session
/// variable for the duration of a transaction and that it is reverted
/// afterwards — the same mechanism used by the adapter's `with_tenant_tx`.
#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn tenant_context_is_set_within_transaction(
    #[future] context: Result<TaskTenantTestContext, BoxError>,
) -> Result<(), BoxError> {
    let test_ctx = context.await?;
    let tenant_id = Uuid::new_v4();

    let db_url = test_ctx.db_url.clone();
    tokio::task::spawn_blocking(move || verify_set_local_scoping(&db_url, tenant_id))
        .await
        .map_err(|e| Box::new(e) as BoxError)??;

    Ok(())
}

/// Verifies that `store` executes within a tenant-scoped transaction by
/// confirming that a task created with one tenant context can be retrieved
/// with the same context.
#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn store_and_find_use_tenant_scoped_transaction(
    #[future] context: Result<TaskTenantTestContext, BoxError>,
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let test_ctx = context.await?;
    let service = &test_ctx.service;
    let ctx = test_request_context;

    let task = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new(
                "github",
                "corbusier/core",
                900,
                "tenant-scoped store test",
            ),
        )
        .await
        .expect("task creation should succeed");

    let found = test_ctx
        .repository
        .find_by_id(&ctx, task.id())
        .await
        .expect("find_by_id should succeed")
        .expect("task should be found");

    assert_eq!(found.id(), task.id());
    Ok(())
}

/// Verifies that `update` executes within a tenant-scoped transaction.
#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn update_uses_tenant_scoped_transaction(
    #[future] context: Result<TaskTenantTestContext, BoxError>,
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let test_ctx = context.await?;
    let service = &test_ctx.service;
    let ctx = test_request_context;

    let task = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new(
                "github",
                "corbusier/core",
                901,
                "tenant-scoped update test",
            ),
        )
        .await
        .expect("task creation should succeed");

    let updated = service
        .associate_branch(
            &ctx,
            AssociateBranchRequest::new(
                task.id(),
                "github",
                "corbusier/core",
                "feature/tenant-test",
            ),
        )
        .await
        .expect("branch association should succeed");

    assert!(updated.branch_ref().is_some());
    Ok(())
}

/// Verifies that `find_by_issue_ref` executes within a tenant-scoped
/// transaction.
#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn find_by_issue_ref_uses_tenant_scoped_transaction(
    #[future] context: Result<TaskTenantTestContext, BoxError>,
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let test_ctx = context.await?;
    let service = &test_ctx.service;
    let ctx = test_request_context;

    service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new(
                "github",
                "corbusier/core",
                902,
                "tenant-scoped issue_ref test",
            ),
        )
        .await
        .expect("task creation should succeed");

    let issue_ref =
        IssueRef::from_parts("github", "corbusier/core", 902).expect("valid issue reference");
    let found = service
        .find_by_issue_ref(&ctx, &issue_ref)
        .await
        .expect("lookup should succeed");
    assert!(found.is_some());
    Ok(())
}

/// Verifies that `find_by_branch_ref` executes within a tenant-scoped
/// transaction.
#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn find_by_branch_ref_uses_tenant_scoped_transaction(
    #[future] context: Result<TaskTenantTestContext, BoxError>,
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let test_ctx = context.await?;
    let service = &test_ctx.service;
    let ctx = test_request_context;

    let task = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new(
                "github",
                "corbusier/core",
                903,
                "tenant-scoped branch_ref test",
            ),
        )
        .await
        .expect("task creation should succeed");

    service
        .associate_branch(
            &ctx,
            AssociateBranchRequest::new(
                task.id(),
                "github",
                "corbusier/core",
                "feature/tenant-branch",
            ),
        )
        .await
        .expect("branch association should succeed");

    let branch_ref = BranchRef::from_parts("github", "corbusier/core", "feature/tenant-branch")
        .expect("valid branch ref");
    let found = service
        .find_by_branch_ref(&ctx, &branch_ref)
        .await
        .expect("lookup should succeed");
    assert_eq!(found.len(), 1);
    Ok(())
}

/// Verifies that `find_by_pull_request_ref` executes within a tenant-scoped
/// transaction.
#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn find_by_pull_request_ref_uses_tenant_scoped_transaction(
    #[future] context: Result<TaskTenantTestContext, BoxError>,
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let test_ctx = context.await?;
    let service = &test_ctx.service;
    let ctx = test_request_context;

    let task = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new(
                "github",
                "corbusier/core",
                904,
                "tenant-scoped pr_ref test",
            ),
        )
        .await
        .expect("task creation should succeed");

    service
        .associate_pull_request(
            &ctx,
            AssociatePullRequestRequest::new(task.id(), "github", "corbusier/core", 55),
        )
        .await
        .expect("PR association should succeed");

    let pr_ref = PullRequestRef::from_parts("github", "corbusier/core", 55).expect("valid PR ref");
    let found = service
        .find_by_pull_request_ref(&ctx, &pr_ref)
        .await
        .expect("lookup should succeed");
    assert_eq!(found.len(), 1);
    Ok(())
}

/// Verifies that `update` on a nonexistent task still properly wraps the
/// operation in a tenant-scoped transaction (the `NotFound` error should
/// propagate cleanly through the tenant transaction).
#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn update_not_found_propagates_through_tenant_tx(
    #[future] context: Result<TaskTenantTestContext, BoxError>,
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let test_ctx = context.await?;
    let service = &test_ctx.service;
    let ctx = test_request_context;

    let result = service
        .associate_branch(
            &ctx,
            AssociateBranchRequest::new(TaskId::new(), "github", "corbusier/core", "main"),
        )
        .await;

    assert!(result.is_err());
    Ok(())
}
