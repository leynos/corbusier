//! Diesel schema for agent backend orchestration persistence.

diesel::table! {
    /// Agent backend registration records.
    backend_registrations (id) {
        /// Internal backend identifier.
        id -> Uuid,
        /// Tenant identifier owning this backend.
        tenant_id -> Uuid,
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

diesel::table! {
    /// Agent turn-session records.
    agent_turn_sessions (id) {
        /// Internal session identifier.
        id -> Uuid,
        /// Tenant identifier owning this session.
        tenant_id -> Uuid,
        /// Owning backend registration identifier.
        backend_id -> Uuid,
        /// Conversation identifier.
        conversation_id -> Uuid,
        /// Backend-native runtime session identifier.
        #[max_length = 255]
        runtime_session_id -> Varchar,
        /// Lifecycle status (`active`, `reserved`, or `expired`).
        #[max_length = 20]
        status -> Varchar,
        /// Session TTL in seconds.
        ttl_seconds -> BigInt,
        /// Session start timestamp.
        started_at -> Timestamptz,
        /// Last successful turn timestamp.
        last_used_at -> Timestamptz,
        /// Session expiry timestamp.
        expires_at -> Timestamptz,
        /// Session end timestamp when expired.
        ended_at -> Nullable<Timestamptz>,
        /// Number of successful turns in this session.
        turn_count -> BigInt,
    }
}
