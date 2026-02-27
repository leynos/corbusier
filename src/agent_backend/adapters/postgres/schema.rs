//! Diesel schema for agent backend registration persistence.

diesel::table! {
    /// Agent backend registration records.
    backend_registrations (id) {
        /// Internal backend identifier.
        id -> Uuid,
        /// Unique human-readable backend name.
        #[max_length = 100]
        name -> Varchar,
        /// Lifecycle status (active or inactive).
        #[max_length = 50]
        status -> Varchar,
        /// Capability metadata as JSONB.
        capabilities -> Jsonb,
        /// Provider information as JSONB.
        backend_info -> Jsonb,
        /// Creation timestamp.
        created_at -> Timestamptz,
        /// Last update timestamp.
        updated_at -> Timestamptz,
    }
}
