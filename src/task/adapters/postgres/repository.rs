//! `PostgreSQL` repository implementation for task lifecycle storage.

use super::{
    models::{NewTaskRow, TaskRow},
    schema::tasks,
};
use crate::task::{
    domain::{
        IssueRef, ParseTaskStateError, PersistedTaskData, Task, TaskId, TaskOrigin, TaskState,
    },
    ports::{TaskRepository, TaskRepositoryError, TaskRepositoryResult},
};
use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use serde_json::Value;

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
            // Pre-check issue reference to provide stable semantic errors.
            let duplicate_issue = find_task_by_issue_ref(connection, &issue_ref)?;
            if duplicate_issue.is_some() {
                return Err(TaskRepositoryError::DuplicateIssueOrigin(issue_ref));
            }

            diesel::insert_into(tasks::table)
                .values(&new_row)
                .execute(connection)
                .map_err(|err| match err {
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
        state: task.state().as_str().to_owned(),
        created_at: task.created_at(),
        updated_at: task.updated_at(),
    })
}

fn row_to_task(row: TaskRow) -> TaskRepositoryResult<Task> {
    let origin = serde_json::from_value::<TaskOrigin>(row.origin)
        .map_err(TaskRepositoryError::persistence)?;
    let state = TaskState::try_from(row.state.as_str())
        .map_err(|err: ParseTaskStateError| TaskRepositoryError::persistence(err))?;

    let data = PersistedTaskData::new(
        TaskId::from_uuid(row.id),
        origin,
        state,
        row.created_at,
        row.updated_at,
    );
    Ok(Task::from_persisted(data))
}

#[derive(Debug, QueryableByName)]
struct LookupTaskRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    origin: Value,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    state: String,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    created_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    updated_at: chrono::DateTime<chrono::Utc>,
}

fn find_task_by_issue_ref(
    connection: &mut PgConnection,
    issue_ref: &IssueRef,
) -> TaskRepositoryResult<Option<TaskRow>> {
    let issue_number = i64::try_from(issue_ref.issue_number().value())
        .map_err(TaskRepositoryError::persistence)?;
    let query = diesel::sql_query(concat!(
        "SELECT id, origin, state, created_at, updated_at FROM tasks ",
        "WHERE origin->>'type' = 'issue' ",
        "AND origin->'issue_ref'->>'provider' = $1 ",
        "AND origin->'issue_ref'->>'repository' = $2 ",
        "AND (origin->'issue_ref'->>'issue_number')::BIGINT = $3 ",
        "LIMIT 1",
    ))
    .bind::<diesel::sql_types::Text, _>(issue_ref.provider().as_str())
    .bind::<diesel::sql_types::Text, _>(issue_ref.repository().as_str())
    .bind::<diesel::sql_types::BigInt, _>(issue_number);

    let row = query
        .get_result::<LookupTaskRow>(connection)
        .optional()
        .map_err(TaskRepositoryError::persistence)?;

    Ok(row.map(|lookup| TaskRow {
        id: lookup.id,
        origin: lookup.origin,
        state: lookup.state,
        created_at: lookup.created_at,
        updated_at: lookup.updated_at,
    }))
}
