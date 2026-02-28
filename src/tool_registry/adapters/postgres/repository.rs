//! `PostgreSQL` repository implementation for MCP server registrations.

use super::{
    models::{McpServerRow, NewMcpServerRow},
    schema::mcp_servers,
};
use crate::tool_registry::{
    domain::{
        McpServerHealthSnapshot, McpServerHealthStatus, McpServerId, McpServerLifecycleState,
        McpServerName, McpServerRegistration, McpTransport, PersistedMcpServerData,
    },
    ports::{McpServerRegistryError, McpServerRegistryRepository, McpServerRegistryResult},
};
use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::{DatabaseErrorKind, Error as DieselError};

/// `PostgreSQL` connection pool type for MCP server adapters.
pub type McpServerPgPool = Pool<ConnectionManager<PgConnection>>;

/// `PostgreSQL`-backed repository for MCP server registry records.
#[derive(Debug, Clone)]
pub struct PostgresMcpServerRegistry {
    pool: McpServerPgPool,
}

impl PostgresMcpServerRegistry {
    /// Creates a new repository from a `PostgreSQL` pool.
    #[must_use]
    pub const fn new(pool: McpServerPgPool) -> Self {
        Self { pool }
    }

    async fn run_blocking<F, T>(&self, operation: F) -> McpServerRegistryResult<T>
    where
        F: FnOnce(&mut PgConnection) -> McpServerRegistryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = pool.get().map_err(McpServerRegistryError::persistence)?;
            operation(&mut connection)
        })
        .await
        .map_err(McpServerRegistryError::persistence)?
    }
}

#[async_trait]
impl McpServerRegistryRepository for PostgresMcpServerRegistry {
    async fn register(&self, server: &McpServerRegistration) -> McpServerRegistryResult<()> {
        let server_id = server.id();
        let server_name = server.name().clone();
        let new_row = to_new_row(server)?;

        self.run_blocking(move |connection| {
            diesel::insert_into(mcp_servers::table)
                .values(&new_row)
                .execute(connection)
                .map_err(|err| match err {
                    DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, ref info)
                        if is_name_unique_violation(info.as_ref()) =>
                    {
                        McpServerRegistryError::DuplicateServerName(server_name.clone())
                    }
                    DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                        McpServerRegistryError::DuplicateServer(server_id)
                    }
                    _ => McpServerRegistryError::persistence(err),
                })?;
            Ok(())
        })
        .await
    }

    async fn update(&self, server: &McpServerRegistration) -> McpServerRegistryResult<()> {
        let server_id = server.id().into_inner();
        let serialized = serialize_server_fields(server)?;

        self.run_blocking(move |connection| {
            let updated_count =
                diesel::update(mcp_servers::table.filter(mcp_servers::id.eq(server_id)))
                    .set((
                        mcp_servers::transport.eq(&serialized.transport),
                        mcp_servers::lifecycle_state.eq(&serialized.lifecycle_state),
                        mcp_servers::health_status.eq(&serialized.health_status),
                        mcp_servers::health_message.eq(&serialized.health_message),
                        mcp_servers::health_checked_at.eq(serialized.health_checked_at),
                        mcp_servers::updated_at.eq(serialized.updated_at),
                    ))
                    .execute(connection)
                    .map_err(McpServerRegistryError::persistence)?;

            if updated_count == 0 {
                return Err(McpServerRegistryError::NotFound(McpServerId::from_uuid(
                    server_id,
                )));
            }
            Ok(())
        })
        .await
    }

    async fn find_by_id(
        &self,
        server_id: McpServerId,
    ) -> McpServerRegistryResult<Option<McpServerRegistration>> {
        self.run_blocking(move |connection| {
            let row = mcp_servers::table
                .filter(mcp_servers::id.eq(server_id.into_inner()))
                .select(McpServerRow::as_select())
                .first::<McpServerRow>(connection)
                .optional()
                .map_err(McpServerRegistryError::persistence)?;
            row.map(row_to_server).transpose()
        })
        .await
    }

    async fn find_by_name(
        &self,
        server_name: &McpServerName,
    ) -> McpServerRegistryResult<Option<McpServerRegistration>> {
        let name = server_name.as_str().to_owned();
        self.run_blocking(move |connection| {
            let row = mcp_servers::table
                .filter(mcp_servers::name.eq(&name))
                .select(McpServerRow::as_select())
                .first::<McpServerRow>(connection)
                .optional()
                .map_err(McpServerRegistryError::persistence)?;
            row.map(row_to_server).transpose()
        })
        .await
    }

    async fn list_all(&self) -> McpServerRegistryResult<Vec<McpServerRegistration>> {
        self.run_blocking(move |connection| {
            let rows = mcp_servers::table
                .select(McpServerRow::as_select())
                .load::<McpServerRow>(connection)
                .map_err(McpServerRegistryError::persistence)?;
            rows.into_iter().map(row_to_server).collect()
        })
        .await
    }
}

struct SerializedServerFields {
    transport: serde_json::Value,
    lifecycle_state: String,
    health_status: String,
    health_message: Option<String>,
    health_checked_at: Option<chrono::DateTime<chrono::Utc>>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

fn serialize_server_fields(
    server: &McpServerRegistration,
) -> McpServerRegistryResult<SerializedServerFields> {
    let transport =
        serde_json::to_value(server.transport()).map_err(McpServerRegistryError::persistence)?;
    let (health_status, health_message, health_checked_at) = serialize_health(server.last_health());

    Ok(SerializedServerFields {
        transport,
        lifecycle_state: server.lifecycle_state().as_str().to_owned(),
        health_status,
        health_message,
        health_checked_at,
        updated_at: server.updated_at(),
    })
}

fn to_new_row(server: &McpServerRegistration) -> McpServerRegistryResult<NewMcpServerRow> {
    let serialized = serialize_server_fields(server)?;

    Ok(NewMcpServerRow {
        id: server.id().into_inner(),
        name: server.name().as_str().to_owned(),
        transport: serialized.transport,
        lifecycle_state: serialized.lifecycle_state,
        health_status: serialized.health_status,
        health_message: serialized.health_message,
        health_checked_at: serialized.health_checked_at,
        created_at: server.created_at(),
        updated_at: serialized.updated_at,
    })
}

fn serialize_health(
    snapshot: Option<&McpServerHealthSnapshot>,
) -> (
    String,
    Option<String>,
    Option<chrono::DateTime<chrono::Utc>>,
) {
    snapshot.map_or_else(
        || (String::from("unknown"), None, None),
        |health| {
            (
                health.status().as_str().to_owned(),
                health.message().map(str::to_owned),
                Some(health.checked_at()),
            )
        },
    )
}

fn row_to_server(row: McpServerRow) -> McpServerRegistryResult<McpServerRegistration> {
    let McpServerRow {
        id,
        name,
        transport,
        lifecycle_state,
        health_status,
        health_message,
        health_checked_at,
        created_at,
        updated_at,
    } = row;

    let parsed_name =
        McpServerName::new(&name).map_err(McpServerRegistryError::invalid_persisted_data)?;
    let parsed_transport: McpTransport = serde_json::from_value(transport)
        .map_err(McpServerRegistryError::invalid_persisted_data)?;
    let parsed_lifecycle = McpServerLifecycleState::try_from(lifecycle_state.as_str())
        .map_err(McpServerRegistryError::invalid_persisted_data)?;
    let parsed_health =
        build_health_snapshot(health_status.as_str(), health_checked_at, health_message)?;

    let data = PersistedMcpServerData {
        id: McpServerId::from_uuid(id),
        name: parsed_name,
        transport: parsed_transport,
        lifecycle_state: parsed_lifecycle,
        last_health: parsed_health,
        created_at,
        updated_at,
    };

    Ok(McpServerRegistration::from_persisted(data))
}

fn build_health_snapshot(
    health_status: &str,
    health_checked_at: Option<chrono::DateTime<chrono::Utc>>,
    health_message: Option<String>,
) -> McpServerRegistryResult<Option<McpServerHealthSnapshot>> {
    let parsed_status = McpServerHealthStatus::try_from(health_status)
        .map_err(McpServerRegistryError::invalid_persisted_data)?;

    let Some(checked_at) = health_checked_at else {
        return Ok(None);
    };

    let snapshot = McpServerHealthSnapshot::new(parsed_status, checked_at);
    Ok(Some(match health_message {
        Some(message) => snapshot.with_message(message),
        None => snapshot,
    }))
}

fn is_name_unique_violation(info: &dyn diesel::result::DatabaseErrorInformation) -> bool {
    info.constraint_name()
        .is_some_and(|name| name == "idx_mcp_servers_name")
}
