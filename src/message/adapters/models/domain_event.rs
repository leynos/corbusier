//! Diesel models for domain event persistence.
//!
//! Maps database rows to Rust structs for the `domain_events` table.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;
use uuid::Uuid;

use super::super::schema::domain_events;

/// Database row representation of a domain event.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = domain_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DomainEventRow {
    /// Unique event identifier.
    pub id: Uuid,
    /// The aggregate this event applies to.
    pub aggregate_id: Uuid,
    /// Type of aggregate.
    pub aggregate_type: String,
    /// Type of event.
    pub event_type: String,
    /// Event payload.
    pub event_data: Value,
    /// Schema version.
    pub event_version: i32,
    /// When the event occurred.
    pub occurred_at: DateTime<Utc>,
    /// Correlation ID for tracing.
    pub correlation_id: Option<Uuid>,
    /// Causation ID.
    pub causation_id: Option<Uuid>,
    /// User who caused the event.
    pub user_id: Option<Uuid>,
    /// Session context.
    pub session_id: Option<Uuid>,
}

/// Data for inserting a new domain event.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = domain_events)]
pub struct NewDomainEvent {
    /// Unique event identifier.
    pub id: Uuid,
    /// The aggregate this event applies to.
    pub aggregate_id: Uuid,
    /// Type of aggregate.
    pub aggregate_type: String,
    /// Type of event.
    pub event_type: String,
    /// Event payload.
    pub event_data: Value,
    /// Schema version.
    pub event_version: i32,
    /// When the event occurred.
    pub occurred_at: DateTime<Utc>,
    /// Correlation ID for tracing.
    pub correlation_id: Option<Uuid>,
    /// Causation ID.
    pub causation_id: Option<Uuid>,
    /// User who caused the event.
    pub user_id: Option<Uuid>,
    /// Session context.
    pub session_id: Option<Uuid>,
}
