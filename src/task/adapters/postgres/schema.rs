//! Diesel schema for task lifecycle persistence.

diesel::table! {
    /// Task records with issue-origin metadata.
    tasks (id) {
        /// Internal task identifier.
        id -> Uuid,
        /// Origin payload including issue reference and metadata.
        origin -> Jsonb,
        /// Optional branch reference linked to this task.
        #[max_length = 255]
        branch_ref -> Nullable<Varchar>,
        /// Optional pull-request reference linked to this task.
        #[max_length = 255]
        pull_request_ref -> Nullable<Varchar>,
        /// Task lifecycle state.
        #[max_length = 50]
        state -> Varchar,
        /// Optional workspace identifier.
        workspace_id -> Nullable<Uuid>,
        /// Creation timestamp.
        created_at -> Timestamptz,
        /// Last update timestamp.
        updated_at -> Timestamptz,
    }
}
