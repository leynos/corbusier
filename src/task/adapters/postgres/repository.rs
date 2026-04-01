//! `PostgreSQL` repository implementation for task lifecycle storage.
//!
//! Tenant context is propagated via `SET LOCAL app.tenant_id`, which sets a
//! `PostgreSQL` session variable scoped to the current transaction.  This
//! prepares the connection for Row-Level Security (RLS) policies but does
//! not enforce row isolation by itself; actual enforcement requires RLS
//! policies on the `tasks` table, which land in milestone 1.5.3.

use super::{
    models::{NewTaskRow, TaskRow},
    schema::tasks,
};
use crate::context::{RequestContext, TenantId};
use crate::message::adapters::postgres::blocking_helpers::{
    PgPool, get_conn_with, run_blocking_with,
};
use crate::postgres_support::{
    FromTxError, TxError, ensure_tenant_exists, with_tenant_read_tx, with_tenant_tx,
};
use crate::task::{
    domain::{
        BranchRef, IssueRef, PersistedTaskData, PullRequestRef, Task, TaskId, TaskOrigin, TaskState,
    },
    ports::{TaskRepository, TaskRepositoryError, TaskRepositoryResult},
};
use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorInformation, DatabaseErrorKind, Error as DieselError};

/// `PostgreSQL` connection pool type used by task adapters.
pub type TaskPgPool = PgPool;

async fn run_tenant_query<F, T, Q>(
    pool: &TaskPgPool,
    tenant_id: TenantId,
    query_fn: F,
    run_query: Q,
) -> TaskRepositoryResult<T>
where
    F: FnOnce(&mut PgConnection) -> TaskRepositoryResult<T> + Send + 'static,
    T: Send + 'static,
    Q: FnOnce(&mut PgConnection, uuid::Uuid, F) -> TaskRepositoryResult<T> + Send + 'static,
{
    let pool_clone = pool.clone();
    let tenant_uuid = tenant_id.into_inner();

    run_blocking_with(
        move || {
            let mut conn = get_conn_with(&pool_clone, TaskRepositoryError::persistence)?;
            run_query(&mut conn, tenant_uuid, query_fn)
        },
        TaskRepositoryError::persistence,
    )
    .await
}

// ---------------------------------------------------------------------------
// Error bridging for the shared transaction helper
// ---------------------------------------------------------------------------

impl FromTxError<Self> for TaskRepositoryError {
    fn from_tx_error(err: TxError<Self>) -> Self {
        match err {
            TxError::Domain(e) => e,
            TxError::Diesel(e) => Self::persistence(e),
        }
    }
}

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// `PostgreSQL`-backed task repository.
#[derive(Debug, Clone)]
pub struct PostgresTaskRepository {
    pool: TaskPgPool,
}

impl PostgresTaskRepository {
    /// Creates a new repository from a `PostgreSQL` connection pool.
    #[must_use]
    #[rustfmt::skip]
    pub const fn new(pool: TaskPgPool) -> Self { Self { pool } }

    /// Executes a write query inside a transaction with tenant context.
    async fn execute_query<F, T>(&self, tenant_id: TenantId, query_fn: F) -> TaskRepositoryResult<T>
    where
        F: FnOnce(&mut PgConnection) -> TaskRepositoryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        run_tenant_query(&self.pool, tenant_id, query_fn, with_tenant_tx).await
    }

    /// Executes a write query that may create the tenant row before use.
    async fn execute_query_with_bootstrap<F, T>(
        &self,
        tenant_id: TenantId,
        query_fn: F,
    ) -> TaskRepositoryResult<T>
    where
        F: FnOnce(&mut PgConnection) -> TaskRepositoryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        run_tenant_query(
            &self.pool,
            tenant_id,
            query_fn,
            |conn, tenant_uuid, run_query| {
                with_tenant_tx(conn, tenant_uuid, |tx| {
                    ensure_tenant_exists(tx, tenant_uuid)
                        .map_err(TaskRepositoryError::persistence)?;
                    run_query(tx)
                })
            },
        )
        .await
    }

    /// Executes a read-only query inside a tenant-scoped transaction.
    ///
    /// This delegates to `run_tenant_query(&self.pool, tenant_id, query_fn,
    /// with_tenant_read_tx)`, and `with_tenant_read_tx` issues
    /// `SET TRANSACTION READ ONLY` before setting tenant context so the
    /// database rejects accidental writes on this path.
    async fn execute_read_query<F, T>(
        &self,
        tenant_id: TenantId,
        query_fn: F,
    ) -> TaskRepositoryResult<T>
    where
        F: FnOnce(&mut PgConnection) -> TaskRepositoryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        run_tenant_query(&self.pool, tenant_id, query_fn, with_tenant_read_tx).await
    }
}

/// Generates a `find_by_*_ref` body that filters `tasks` by a nullable
/// `VARCHAR` column and maps the resulting rows to domain `Task` values.
macro_rules! find_tasks_by_ref_column {
    ($self:expr, $tenant_id:expr, $ref_str:expr, $column:expr) => {{
        let tenant_for_query = $tenant_id;
        let value = $ref_str;
        $self
            .execute_read_query(tenant_for_query, move |conn| {
                let rows = tasks::table
                    .filter(tasks::tenant_id.eq(tenant_for_query.into_inner()))
                    .filter($column.eq(&value))
                    .select(TaskRow::as_select())
                    .load::<TaskRow>(conn)
                    .map_err(TaskRepositoryError::persistence)?;
                rows.into_iter().map(row_to_task).collect()
            })
            .await
    }};
}

#[async_trait]
impl TaskRepository for PostgresTaskRepository {
    async fn store(&self, ctx: &RequestContext, task: &Task) -> TaskRepositoryResult<()> {
        let tenant_id = ctx.tenant_id();
        let task_id = task.id();
        let issue_ref = task.origin().issue_ref().clone();
        let new_row = to_new_row(task, tenant_id)?;

        self.execute_query_with_bootstrap(tenant_id, move |conn| {
            // Pre-check for semantic error reporting; the unique index still
            // enforces integrity in the TOCTOU window between check and insert.
            let duplicate_issue = find_task_by_issue_ref(conn, tenant_id, &issue_ref)?;
            if duplicate_issue.is_some() {
                return Err(TaskRepositoryError::DuplicateIssueOrigin(issue_ref.clone()));
            }

            diesel::insert_into(tasks::table)
                .values(&new_row)
                .execute(conn)
                .map_err(|err| match err {
                    DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, ref info)
                        if is_issue_origin_unique_violation(info.as_ref()) =>
                    {
                        TaskRepositoryError::DuplicateIssueOrigin(issue_ref.clone())
                    }
                    DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                        TaskRepositoryError::DuplicateTask(task_id)
                    }
                    _ => TaskRepositoryError::persistence(err),
                })?;

            Ok(())
        })
        .await
    }

    async fn update(&self, ctx: &RequestContext, task: &Task) -> TaskRepositoryResult<()> {
        let tenant_id = ctx.tenant_id();
        let task_id = task.id().into_inner();
        let branch_val = task.branch_ref().map(ToString::to_string);
        let pr_val = task.pull_request_ref().map(ToString::to_string);
        let state_val = task.state().as_str().to_owned();
        let updated_val = task.updated_at();

        self.execute_query(tenant_id, move |conn| {
            let updated_count = diesel::update(
                tasks::table
                    .filter(tasks::id.eq(task_id))
                    .filter(tasks::tenant_id.eq(tenant_id.into_inner())),
            )
            .set((
                tasks::branch_ref.eq(&branch_val),
                tasks::pull_request_ref.eq(&pr_val),
                tasks::state.eq(&state_val),
                tasks::updated_at.eq(updated_val),
            ))
            .execute(conn)
            .map_err(TaskRepositoryError::persistence)?;

            if updated_count == 0 {
                return Err(TaskRepositoryError::NotFound(TaskId::from_uuid(task_id)));
            }
            Ok(())
        })
        .await
    }

    async fn find_by_id(
        &self,
        ctx: &RequestContext,
        id: TaskId,
    ) -> TaskRepositoryResult<Option<Task>> {
        let tenant_id = ctx.tenant_id();

        self.execute_read_query(tenant_id, move |conn| {
            let row = tasks::table
                .filter(tasks::id.eq(id.into_inner()))
                .filter(tasks::tenant_id.eq(tenant_id.into_inner()))
                .select(TaskRow::as_select())
                .first::<TaskRow>(conn)
                .optional()
                .map_err(TaskRepositoryError::persistence)?;
            row.map(row_to_task).transpose()
        })
        .await
    }

    async fn find_by_issue_ref(
        &self,
        ctx: &RequestContext,
        issue_ref: &IssueRef,
    ) -> TaskRepositoryResult<Option<Task>> {
        let tenant_id = ctx.tenant_id();
        let lookup_issue_ref = issue_ref.clone();

        self.execute_read_query(tenant_id, move |conn| {
            let row = find_task_by_issue_ref(conn, tenant_id, &lookup_issue_ref)?;
            row.map(row_to_task).transpose()
        })
        .await
    }

    async fn find_by_branch_ref(
        &self,
        ctx: &RequestContext,
        branch_ref: &BranchRef,
    ) -> TaskRepositoryResult<Vec<Task>> {
        let tenant_id = ctx.tenant_id();
        let ref_str = branch_ref.to_string();
        find_tasks_by_ref_column!(self, tenant_id, ref_str, tasks::branch_ref)
    }

    async fn find_by_pull_request_ref(
        &self,
        ctx: &RequestContext,
        pr_ref: &PullRequestRef,
    ) -> TaskRepositoryResult<Vec<Task>> {
        let tenant_id = ctx.tenant_id();
        let ref_str = pr_ref.to_string();
        find_tasks_by_ref_column!(self, tenant_id, ref_str, tasks::pull_request_ref)
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

fn to_new_row(task: &Task, tenant_id: TenantId) -> TaskRepositoryResult<NewTaskRow> {
    let origin = serde_json::to_value(task.origin()).map_err(TaskRepositoryError::persistence)?;

    Ok(NewTaskRow {
        id: task.id().into_inner(),
        tenant_id: tenant_id.into_inner(),
        origin,
        branch_ref: task.branch_ref().map(ToString::to_string),
        pull_request_ref: task.pull_request_ref().map(ToString::to_string),
        state: task.state().as_str().to_owned(),
        workspace_id: None,
        created_at: task.created_at(),
        updated_at: task.updated_at(),
    })
}

fn row_to_task(row: TaskRow) -> TaskRepositoryResult<Task> {
    let TaskRow {
        id,
        tenant_id: _tenant_id,
        origin: persisted_origin,
        branch_ref,
        pull_request_ref,
        state: persisted_state,
        workspace_id,
        created_at,
        updated_at,
    } = row;

    // workspace_id is still deferred to roadmap item 1.2.3.
    debug_assert!(
        workspace_id.is_none(),
        "workspace column should remain unset until roadmap item 1.2.3"
    );

    let origin = serde_json::from_value::<TaskOrigin>(persisted_origin)
        .map_err(TaskRepositoryError::persistence)?;
    let state =
        TaskState::try_from(persisted_state.as_str()).map_err(TaskRepositoryError::persistence)?;

    let parsed_branch = branch_ref
        .map(|s| BranchRef::parse_canonical(&s))
        .transpose()
        .map_err(TaskRepositoryError::persistence)?;
    let parsed_pr = pull_request_ref
        .map(|s| PullRequestRef::parse_canonical(&s))
        .transpose()
        .map_err(TaskRepositoryError::persistence)?;

    let data = PersistedTaskData {
        id: TaskId::from_uuid(id),
        origin,
        branch_ref: parsed_branch,
        pull_request_ref: parsed_pr,
        state,
        created_at,
        updated_at,
    };
    Ok(Task::from_persisted(data))
}

// ---------------------------------------------------------------------------
// Constraint helpers
// ---------------------------------------------------------------------------

fn is_issue_origin_unique_violation(info: &dyn DatabaseErrorInformation) -> bool {
    info.constraint_name()
        .is_some_and(|name| name == "idx_tasks_issue_origin_unique")
}

fn find_task_by_issue_ref(
    connection: &mut PgConnection,
    tenant_id: TenantId,
    issue_ref: &IssueRef,
) -> TaskRepositoryResult<Option<TaskRow>> {
    let issue_number = i64::try_from(issue_ref.issue_number().value())
        .map_err(TaskRepositoryError::persistence)?;
    let query = diesel::sql_query(concat!(
        "SELECT id, tenant_id, origin, branch_ref, pull_request_ref, state, workspace_id, ",
        "created_at, updated_at FROM tasks ",
        "WHERE origin->>'type' = 'issue' ",
        "AND tenant_id = $1 ",
        "AND origin->'issue_ref'->>'provider' = $2 ",
        "AND origin->'issue_ref'->>'repository' = $3 ",
        "AND (origin->'issue_ref'->>'issue_number')::BIGINT = $4 ",
        "LIMIT 1",
    ))
    .bind::<diesel::sql_types::Uuid, _>(tenant_id.into_inner())
    .bind::<diesel::sql_types::Text, _>(issue_ref.provider().as_str())
    .bind::<diesel::sql_types::Text, _>(issue_ref.repository().as_str())
    .bind::<diesel::sql_types::BigInt, _>(issue_number);

    query
        .get_result::<TaskRow>(connection)
        .optional()
        .map_err(TaskRepositoryError::persistence)
}
