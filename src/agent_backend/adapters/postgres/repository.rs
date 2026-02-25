//! `PostgreSQL` repository implementation for agent backend registration.

use super::{
    models::{BackendRegistrationRow, NewBackendRegistrationRow},
    schema::backend_registrations,
};
use crate::agent_backend::{
    domain::{
        AgentBackendRegistration, AgentCapabilities, BackendId, BackendInfo, BackendName,
        BackendStatus, PersistedBackendData,
    },
    ports::{BackendRegistryError, BackendRegistryRepository, BackendRegistryResult},
};
use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::{DatabaseErrorKind, Error as DieselError};

/// `PostgreSQL` connection pool type used by backend registry adapters.
pub type BackendPgPool = Pool<ConnectionManager<PgConnection>>;

/// `PostgreSQL`-backed backend registry repository.
#[derive(Debug, Clone)]
pub struct PostgresBackendRegistry {
    pool: BackendPgPool,
}

impl PostgresBackendRegistry {
    /// Creates a new repository from a `PostgreSQL` connection pool.
    #[must_use]
    pub const fn new(pool: BackendPgPool) -> Self {
        Self { pool }
    }

    async fn run_blocking<F, T>(&self, f: F) -> BackendRegistryResult<T>
    where
        F: FnOnce(&mut PgConnection) -> BackendRegistryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = pool.get().map_err(BackendRegistryError::persistence)?;
            f(&mut connection)
        })
        .await
        .map_err(BackendRegistryError::persistence)?
    }
}

#[async_trait]
impl BackendRegistryRepository for PostgresBackendRegistry {
    async fn register(&self, registration: &AgentBackendRegistration) -> BackendRegistryResult<()> {
        let backend_id = registration.id();
        let backend_name = registration.name().clone();
        let new_row = to_new_row(registration)?;

        self.run_blocking(move |connection| {
            diesel::insert_into(backend_registrations::table)
                .values(&new_row)
                .execute(connection)
                .map_err(|err| match err {
                    DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, ref info)
                        if is_name_unique_violation(info.as_ref()) =>
                    {
                        BackendRegistryError::DuplicateBackendName(backend_name.clone())
                    }
                    DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                        BackendRegistryError::DuplicateBackend(backend_id)
                    }
                    _ => BackendRegistryError::persistence(err),
                })?;
            Ok(())
        })
        .await
    }

    async fn update(&self, registration: &AgentBackendRegistration) -> BackendRegistryResult<()> {
        let backend_id = registration.id().into_inner();
        let status_val = registration.status().as_str().to_owned();
        let capabilities_val = serde_json::to_value(registration.capabilities())
            .map_err(BackendRegistryError::persistence)?;
        let backend_info_val = serde_json::to_value(registration.backend_info())
            .map_err(BackendRegistryError::persistence)?;
        let updated_val = registration.updated_at();

        self.run_blocking(move |connection| {
            let updated_count = diesel::update(
                backend_registrations::table.filter(backend_registrations::id.eq(backend_id)),
            )
            .set((
                backend_registrations::status.eq(&status_val),
                backend_registrations::capabilities.eq(&capabilities_val),
                backend_registrations::backend_info.eq(&backend_info_val),
                backend_registrations::updated_at.eq(updated_val),
            ))
            .execute(connection)
            .map_err(BackendRegistryError::persistence)?;

            if updated_count == 0 {
                return Err(BackendRegistryError::NotFound(BackendId::from_uuid(
                    backend_id,
                )));
            }
            Ok(())
        })
        .await
    }

    async fn find_by_id(
        &self,
        id: BackendId,
    ) -> BackendRegistryResult<Option<AgentBackendRegistration>> {
        self.run_blocking(move |connection| {
            let row = backend_registrations::table
                .filter(backend_registrations::id.eq(id.into_inner()))
                .select(BackendRegistrationRow::as_select())
                .first::<BackendRegistrationRow>(connection)
                .optional()
                .map_err(BackendRegistryError::persistence)?;
            row.map(row_to_registration).transpose()
        })
        .await
    }

    async fn find_by_name(
        &self,
        name: &BackendName,
    ) -> BackendRegistryResult<Option<AgentBackendRegistration>> {
        let name_str = name.as_str().to_owned();
        self.run_blocking(move |connection| {
            let row = backend_registrations::table
                .filter(backend_registrations::name.eq(&name_str))
                .select(BackendRegistrationRow::as_select())
                .first::<BackendRegistrationRow>(connection)
                .optional()
                .map_err(BackendRegistryError::persistence)?;
            row.map(row_to_registration).transpose()
        })
        .await
    }

    async fn list_active(&self) -> BackendRegistryResult<Vec<AgentBackendRegistration>> {
        self.run_blocking(move |connection| {
            let rows = backend_registrations::table
                .filter(backend_registrations::status.eq("active"))
                .select(BackendRegistrationRow::as_select())
                .load::<BackendRegistrationRow>(connection)
                .map_err(BackendRegistryError::persistence)?;
            rows.into_iter().map(row_to_registration).collect()
        })
        .await
    }

    async fn list_all(&self) -> BackendRegistryResult<Vec<AgentBackendRegistration>> {
        self.run_blocking(move |connection| {
            let rows = backend_registrations::table
                .select(BackendRegistrationRow::as_select())
                .load::<BackendRegistrationRow>(connection)
                .map_err(BackendRegistryError::persistence)?;
            rows.into_iter().map(row_to_registration).collect()
        })
        .await
    }
}

fn to_new_row(
    registration: &AgentBackendRegistration,
) -> BackendRegistryResult<NewBackendRegistrationRow> {
    let capabilities = serde_json::to_value(registration.capabilities())
        .map_err(BackendRegistryError::persistence)?;
    let backend_info = serde_json::to_value(registration.backend_info())
        .map_err(BackendRegistryError::persistence)?;

    Ok(NewBackendRegistrationRow {
        id: registration.id().into_inner(),
        name: registration.name().as_str().to_owned(),
        status: registration.status().as_str().to_owned(),
        capabilities,
        backend_info,
        created_at: registration.created_at(),
        updated_at: registration.updated_at(),
    })
}

fn row_to_registration(
    row: BackendRegistrationRow,
) -> BackendRegistryResult<AgentBackendRegistration> {
    let BackendRegistrationRow {
        id,
        name,
        status,
        capabilities,
        backend_info,
        created_at,
        updated_at,
    } = row;

    let parsed_name = BackendName::new(&name).map_err(BackendRegistryError::persistence)?;
    let parsed_status =
        BackendStatus::try_from(status.as_str()).map_err(BackendRegistryError::persistence)?;
    let parsed_capabilities: AgentCapabilities =
        serde_json::from_value(capabilities).map_err(BackendRegistryError::persistence)?;
    let parsed_info: BackendInfo =
        serde_json::from_value(backend_info).map_err(BackendRegistryError::persistence)?;

    let data = PersistedBackendData {
        id: BackendId::from_uuid(id),
        name: parsed_name,
        status: parsed_status,
        capabilities: parsed_capabilities,
        backend_info: parsed_info,
        created_at,
        updated_at,
    };
    Ok(AgentBackendRegistration::from_persisted(data))
}

fn is_name_unique_violation(info: &dyn diesel::result::DatabaseErrorInformation) -> bool {
    info.constraint_name()
        .is_some_and(|name| name == "idx_backend_registrations_name")
}
