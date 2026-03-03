//! Diesel schema for hook execution persistence.

diesel::table! {
    /// Hook execution records with structured action results.
    hook_executions (id) {
        /// Unique execution identifier.
        id -> Uuid,
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
