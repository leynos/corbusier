//! Versioned event types for schema migration support.
//!
//! Events are stored with explicit version numbers, allowing the system
//! to upgrade older events to the current schema on read.

use chrono::{DateTime, Utc};
use mockable::Clock;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A versioned event with schema migration support.
///
/// All domain events are wrapped in this structure to enable forward-compatible
/// schema evolution. The version field determines which upgraders to apply.
///
/// # Examples
///
/// ```
/// use corbusier::message::versioning::VersionedEvent;
/// use serde_json::json;
///
/// let event = VersionedEvent::new(1, "MessageCreated", json!({
///     "id": "123",
///     "content": "Hello"
/// }));
/// assert_eq!(event.version(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedEvent {
    /// The schema version of this event.
    version: u32,
    /// The type of event.
    event_type: String,
    /// The event data as JSON.
    data: Value,
    /// Metadata about the event.
    metadata: EventMetadata,
}

impl VersionedEvent {
    /// Creates a new versioned event with the current timestamp.
    #[must_use]
    pub fn new(version: u32, event_type: impl Into<String>, data: Value) -> Self {
        Self {
            version,
            event_type: event_type.into(),
            data,
            metadata: EventMetadata::now(),
        }
    }

    /// Creates a new versioned event with a custom clock.
    #[must_use]
    pub fn new_with_clock(
        version: u32,
        event_type: impl Into<String>,
        data: Value,
        clock: &impl Clock,
    ) -> Self {
        Self {
            version,
            event_type: event_type.into(),
            data,
            metadata: EventMetadata::with_clock(clock),
        }
    }

    /// Returns the schema version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Returns the event type.
    #[must_use]
    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    /// Returns the event data.
    #[must_use]
    pub const fn data(&self) -> &Value {
        &self.data
    }

    /// Returns a mutable reference to the event data.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "&mut self methods cannot be const in stable Rust"
    )]
    pub fn data_mut(&mut self) -> &mut Value {
        &mut self.data
    }

    /// Returns the event metadata.
    #[must_use]
    pub const fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    /// Sets the schema version.
    ///
    /// This is used by upgraders to bump the version after transformation.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "&mut self methods cannot be const in stable Rust"
    )]
    pub fn set_version(&mut self, version: u32) {
        self.version = version;
    }
}

/// Metadata associated with an event.
///
/// Captures contextual information about when and where the event occurred.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// When the event occurred.
    pub occurred_at: DateTime<Utc>,
    /// The source system that generated the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Correlation ID for distributed tracing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

impl EventMetadata {
    /// Creates metadata with the current timestamp.
    #[must_use]
    pub fn now() -> Self {
        Self {
            occurred_at: Utc::now(),
            source: None,
            correlation_id: None,
        }
    }

    /// Creates metadata using a custom clock.
    #[must_use]
    pub fn with_clock(clock: &impl Clock) -> Self {
        Self {
            occurred_at: clock.utc(),
            source: None,
            correlation_id: None,
        }
    }

    /// Sets the source system.
    #[must_use]
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Sets the correlation ID.
    #[must_use]
    pub fn with_correlation_id(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self::now()
    }
}
