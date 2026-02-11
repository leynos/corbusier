//! `PostgreSQL` repository implementation for task lifecycle storage.

use super::{
    models::{NewTaskRow, TaskRow},
    schema::tasks,
};
use crate::task::{
    domain::{IssueRef, PersistedTaskData, Task, TaskId, TaskOrigin, TaskState},
    ports::{TaskRepository, TaskRepositoryError, TaskRepositoryResult},
};
use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::{DatabaseErrorInformation, DatabaseErrorKind, Error as DieselError};

/// `PostgreSQL` connection pool type used by task adapters.
pub type TaskPgPool = Pool<ConnectionManager<PgConnection>>;

/// `PostgreSQL`-backed task repository.
#[derive(Debug, Clone)]
pub struct PostgresTaskRepository {
    pool: TaskPgPool,
}

impl PostgresTaskRepository {
    /// Creates a new repository from a `PostgreSQL` connection pool.
    #[must_use]
    pub const fn new(pool: TaskPgPool) -> Self {
        Self { pool }
    }

    async fn run_blocking<F, T>(&self, f: F) -> TaskRepositoryResult<T>
    where
        F: FnOnce(&mut PgConnection) -> TaskRepositoryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = pool.get().map_err(TaskRepositoryError::persistence)?;
            f(&mut connection)
        })
        .await
        .map_err(TaskRepositoryError::persistence)?
    }
}

#[async_trait]
impl TaskRepository for PostgresTaskRepository {
    async fn store(&self, task: &Task) -> TaskRepositoryResult<()> {
        let task_id = task.id();
        let issue_ref = task.origin().issue_ref().clone();
        let new_row = to_new_row(task)?;

        self.run_blocking(move |connection| {
            // This pre-check improves semantic error reporting but is not relied
            // on for correctness: the unique index still enforces integrity in
            // the TOCTOU window between check and insert.
            let duplicate_issue = find_task_by_issue_ref(connection, &issue_ref)?;
            if duplicate_issue.is_some() {
                return Err(TaskRepositoryError::DuplicateIssueOrigin(issue_ref.clone()));
            }

            diesel::insert_into(tasks::table)
                .values(&new_row)
                .execute(connection)
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

    async fn find_by_id(&self, id: TaskId) -> TaskRepositoryResult<Option<Task>> {
        self.run_blocking(move |connection| {
            let row = tasks::table
                .filter(tasks::id.eq(id.into_inner()))
                .select(TaskRow::as_select())
                .first::<TaskRow>(connection)
                .optional()
                .map_err(TaskRepositoryError::persistence)?;
            row.map(row_to_task).transpose()
        })
        .await
    }

    async fn find_by_issue_ref(&self, issue_ref: &IssueRef) -> TaskRepositoryResult<Option<Task>> {
        let lookup_issue_ref = issue_ref.clone();
        self.run_blocking(move |connection| {
            let row = find_task_by_issue_ref(connection, &lookup_issue_ref)?;
            row.map(row_to_task).transpose()
        })
        .await
    }
}

fn to_new_row(task: &Task) -> TaskRepositoryResult<NewTaskRow> {
    let origin = serde_json::to_value(task.origin()).map_err(TaskRepositoryError::persistence)?;

    Ok(NewTaskRow {
        id: task.id().into_inner(),
        origin,
        branch_ref: None,
        pull_request_ref: None,
        state: task.state().as_str().to_owned(),
        workspace_id: None,
        created_at: task.created_at(),
        updated_at: task.updated_at(),
    })
}

fn row_to_task(row: TaskRow) -> TaskRepositoryResult<Task> {
    let TaskRow {
        id,
        origin: persisted_origin,
        branch_ref,
        pull_request_ref,
        state: persisted_state,
        workspace_id,
        created_at,
        updated_at,
    } = row;

    let has_deferred_links =
        branch_ref.is_some() || pull_request_ref.is_some() || workspace_id.is_some();
    debug_assert!(
        !has_deferred_links,
        "deferred task link/workspace columns should remain unset until roadmap items 1.2.2 and 1.2.3"
    );

    let origin = serde_json::from_value::<TaskOrigin>(persisted_origin)
        .map_err(TaskRepositoryError::persistence)?;
    let state =
        TaskState::try_from(persisted_state.as_str()).map_err(TaskRepositoryError::persistence)?;

    let data = PersistedTaskData {
        id: TaskId::from_uuid(id),
        origin,
        state,
        created_at,
        updated_at,
    };
    Ok(Task::from_persisted(data))
}

fn is_issue_origin_unique_violation(info: &dyn DatabaseErrorInformation) -> bool {
    info.constraint_name()
        .is_some_and(|name| name == "idx_tasks_issue_origin_unique")
}

fn find_task_by_issue_ref(
    connection: &mut PgConnection,
    issue_ref: &IssueRef,
) -> TaskRepositoryResult<Option<TaskRow>> {
    let issue_number = i64::try_from(issue_ref.issue_number().value())
        .map_err(TaskRepositoryError::persistence)?;
    let query = diesel::sql_query(concat!(
        "SELECT id, origin, branch_ref, pull_request_ref, state, workspace_id, created_at, ",
        "updated_at FROM tasks ",
        "WHERE origin->>'type' = 'issue' ",
        "AND origin->'issue_ref'->>'provider' = $1 ",
        "AND origin->'issue_ref'->>'repository' = $2 ",
        "AND (origin->'issue_ref'->>'issue_number')::BIGINT = $3 ",
        "LIMIT 1",
    ))
    .bind::<diesel::sql_types::Text, _>(issue_ref.provider().as_str())
    .bind::<diesel::sql_types::Text, _>(issue_ref.repository().as_str())
    .bind::<diesel::sql_types::BigInt, _>(issue_number);

    query
        .get_result::<TaskRow>(connection)
        .optional()
        .map_err(TaskRepositoryError::persistence)
}
