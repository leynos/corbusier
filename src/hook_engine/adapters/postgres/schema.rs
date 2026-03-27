//! Diesel schema for hook engine projections.
//!
//! This module defines the `PostgreSQL` tables used to persist hook execution
//! records and hook policy audit projections, including
//! `hook_policy_audit_events`.

diesel::table! {
    /// Hook execution records with structured action results.
    hook_executions (id) {
        /// Unique execution identifier.
        id -> Uuid,
        /// Tenant identifier for row isolation.
        tenant_id -> Uuid,
        /// Trigger context correlation identifier.
        trigger_context_id -> Uuid,
        /// Hook definition identifier.
        hook_id -> Text,
        /// Trigger type identifier.
        #[max_length = 64]
        trigger_type -> Varchar,
        /// Predicate data used to match the hook.
        predicate_data -> Jsonb,
        /// Structured action execution results.
        action_results -> Jsonb,
        /// Overall execution status.
        #[max_length = 32]
        status -> Varchar,
        /// Execution timestamp.
        executed_at -> Timestamptz,
    }
}

diesel::table! {
    /// Hook policy audit projections.
    hook_policy_audit_events (id) {
        /// Unique policy audit event identifier.
        id -> Uuid,
        /// Tenant identifier for row isolation.
        tenant_id -> Uuid,
        /// Hook execution correlation identifier.
        hook_execution_id -> Uuid,
        /// Trigger context correlation identifier.
        trigger_context_id -> Uuid,
        /// Trigger type identifier.
        #[max_length = 64]
        trigger_type -> Varchar,
        /// Hook definition identifier.
        hook_id -> Text,
        /// Hook action identifier.
        action_id -> Text,
        /// Correlated task identifier.
        task_id -> Nullable<Uuid>,
        /// Correlated conversation identifier.
        conversation_id -> Nullable<Uuid>,
        /// Policy decision.
        #[max_length = 32]
        decision -> Varchar,
        /// Optional structured violation payload.
        violation -> Nullable<Jsonb>,
        /// Raw policy payload.
        payload -> Jsonb,
        /// Projection timestamp.
        recorded_at -> Timestamptz,
    }
}
