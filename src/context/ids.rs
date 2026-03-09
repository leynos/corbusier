//! Cross-cutting identifier types.
//!
//! These newtypes provide type safety for the various UUID identifiers that
//! flow through request context, tenant binding, and audit trails.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Defines a UUID newtype with standard constructors, conversions, and derives.
///
/// Each invocation expands into a `#[serde(transparent)]` tuple struct plus
/// implementations of `new()`, `from_uuid()`, `into_inner()`, `Default`,
/// `AsRef<Uuid>`, and `Display`.
macro_rules! define_uuid_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Creates a new random identifier.
            #[must_use]
            pub fn new() -> Self { Self(Uuid::new_v4()) }

            /// Creates an identifier from an existing UUID.
            #[must_use]
            pub const fn from_uuid(uuid: Uuid) -> Self { Self(uuid) }

            /// Returns the wrapped UUID.
            #[must_use]
            pub const fn into_inner(self) -> Uuid { self.0 }
        }

        impl Default for $name {
            fn default() -> Self { Self::new() }
        }

        impl AsRef<Uuid> for $name {
            fn as_ref(&self) -> &Uuid { &self.0 }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

define_uuid_id!(
    /// Unique identifier for a tenant.
    TenantId
);

define_uuid_id!(
    /// Correlation identifier linking operations within a single user request.
    CorrelationId
);

define_uuid_id!(
    /// Causation identifier pointing to the domain event that triggered an
    /// operation.
    CausationId
);

define_uuid_id!(
    /// User identifier for the actor performing an operation.
    UserId
);

define_uuid_id!(
    /// Session identifier for the current user session.
    SessionId
);
