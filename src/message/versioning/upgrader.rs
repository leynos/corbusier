//! Event upgrader trait and implementations for schema migrations.
//!
//! Upgraders transform events from older schema versions to the current
//! version, enabling backwards-compatible evolution of the event format.

use super::VersionedEvent;
use crate::message::error::SchemaUpgradeError;
use std::collections::HashMap;

/// Result type for upgrade operations.
pub type UpgradeResult<T> = Result<T, SchemaUpgradeError>;

/// Trait for upgrading events between schema versions.
///
/// Implementations handle the transformation of event data from one
/// schema version to the next. Upgraders are chained to migrate events
/// through multiple versions if needed.
///
/// # Implementation Notes
///
/// - Upgraders should be stateless and deterministic
/// - The upgrade path should be well-defined (no skipping versions)
/// - Upgraders should preserve all data that can be mapped to the new schema
pub trait EventUpgrader: Send + Sync {
    /// Upgrades an event to the current supported version.
    ///
    /// If the event is already at the current version, returns it unchanged.
    /// If the event is at an older version, applies transformations to bring
    /// it to the current version.
    ///
    /// # Errors
    ///
    /// Returns `SchemaUpgradeError` if:
    /// - The event version is not supported
    /// - The upgrade transformation fails
    /// - The event data is malformed
    fn upgrade(&self, event: VersionedEvent) -> UpgradeResult<VersionedEvent>;

    /// Returns the current (target) version that this upgrader produces.
    fn current_version(&self) -> u32;

    /// Returns `true` if this upgrader can handle the given version.
    fn supports_version(&self, version: u32) -> bool;
}

/// Upgrader for `MessageCreated` events.
///
/// Handles migration of message creation events from v1 to v2:
/// - v1 → v2: Adds the `metadata` field if missing
///
/// # Schema Changes
///
/// **v1 format:**
/// ```json
/// { "id": "...", "content": [...] }
/// ```
///
/// **v2 format:**
/// ```json
/// { "id": "...", "content": [...], "metadata": {} }
/// ```
#[derive(Debug, Default)]
pub struct MessageCreatedUpgrader;

impl MessageCreatedUpgrader {
    /// The current schema version.
    pub const CURRENT_VERSION: u32 = 2;

    /// Supported schema versions.
    const SUPPORTED_VERSIONS: &'static [u32] = &[1, 2];

    /// Creates a new upgrader.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Upgrades a v1 event to v2 by adding the metadata field if missing.
    fn upgrade_v1_to_v2(mut event: VersionedEvent) -> UpgradeResult<VersionedEvent> {
        let obj = event
            .data_mut()
            .as_object_mut()
            .ok_or_else(|| SchemaUpgradeError::malformed("expected event data to be an object"))?;

        if !obj.contains_key("metadata") {
            obj.insert("metadata".to_owned(), serde_json::json!({}));
        }
        event.set_version(2);
        Ok(event)
    }
}

impl EventUpgrader for MessageCreatedUpgrader {
    fn upgrade(&self, event: VersionedEvent) -> UpgradeResult<VersionedEvent> {
        match event.version() {
            1 => Self::upgrade_v1_to_v2(event),
            2 => Ok(event), // Current version, no upgrade needed
            v => Err(SchemaUpgradeError::UnsupportedVersion(v)),
        }
    }

    fn current_version(&self) -> u32 {
        Self::CURRENT_VERSION
    }

    fn supports_version(&self, version: u32) -> bool {
        Self::SUPPORTED_VERSIONS.contains(&version)
    }
}

/// Registry of event upgraders keyed by event type.
///
/// The registry dispatches upgrade requests to the appropriate upgrader
/// based on the event type.
///
/// # Examples
///
/// ```
/// use corbusier::message::versioning::{UpgraderRegistry, VersionedEvent};
/// use serde_json::json;
///
/// let registry = UpgraderRegistry::new();
/// let event = VersionedEvent::new(1, "MessageCreated", json!({"id": "123"}));
/// let upgraded = registry.upgrade(event).expect("should upgrade");
/// assert_eq!(upgraded.version(), 2);
/// ```
#[derive(Default)]
pub struct UpgraderRegistry {
    upgraders: HashMap<String, Box<dyn EventUpgrader>>,
}

impl UpgraderRegistry {
    /// Creates a new registry with default upgraders.
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register("MessageCreated", Box::new(MessageCreatedUpgrader::new()));
        registry
    }

    /// Creates an empty registry.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Registers an upgrader for an event type.
    ///
    /// If an upgrader is already registered for the event type, it is replaced.
    pub fn register(&mut self, event_type: &str, upgrader: Box<dyn EventUpgrader>) {
        self.upgraders.insert(event_type.to_owned(), upgrader);
    }

    /// Upgrades an event using the appropriate upgrader.
    ///
    /// # Errors
    ///
    /// - Returns `SchemaUpgradeError::UnknownEventType` if no upgrader is
    ///   registered for the event type.
    /// - May return `SchemaUpgradeError::UnsupportedVersion` if the upgrader
    ///   for the event type does not support the event's version.
    pub fn upgrade(&self, event: VersionedEvent) -> UpgradeResult<VersionedEvent> {
        match self.upgraders.get(event.event_type()) {
            Some(upgrader) => upgrader.upgrade(event),
            None => Err(SchemaUpgradeError::UnknownEventType(
                event.event_type().to_owned(),
            )),
        }
    }

    /// Returns `true` if an upgrader is registered for the event type.
    #[must_use]
    pub fn has_upgrader(&self, event_type: &str) -> bool {
        self.upgraders.contains_key(event_type)
    }

    /// Returns the current version for an event type, if known.
    #[must_use]
    pub fn current_version(&self, event_type: &str) -> Option<u32> {
        self.upgraders.get(event_type).map(|u| u.current_version())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn message_created_upgrader_v1_to_v2() {
        let sut = MessageCreatedUpgrader::new();
        let event = VersionedEvent::new(1, "MessageCreated", json!({"id": "123", "content": []}));

        let result = sut.upgrade(event).expect("should upgrade");

        assert_eq!(result.version(), 2);
        assert!(result.data().get("metadata").is_some());
    }

    #[test]
    fn message_created_upgrader_v2_unchanged() {
        let sut = MessageCreatedUpgrader::new();
        let event = VersionedEvent::new(
            2,
            "MessageCreated",
            json!({"id": "123", "content": [], "metadata": {"key": "value"}}),
        );

        let result = sut.upgrade(event).expect("should upgrade");

        assert_eq!(result.version(), 2);
        assert_eq!(
            result.data().get("metadata"),
            Some(&json!({"key": "value"}))
        );
    }

    #[test]
    fn message_created_upgrader_unsupported_version() {
        let upgrader = MessageCreatedUpgrader::new();
        let event = VersionedEvent::new(99, "MessageCreated", json!({}));

        let result = upgrader.upgrade(event);

        assert!(matches!(
            result,
            Err(SchemaUpgradeError::UnsupportedVersion(99))
        ));
    }

    #[test]
    fn registry_dispatches_to_correct_upgrader() {
        let registry = UpgraderRegistry::new();
        let event = VersionedEvent::new(1, "MessageCreated", json!({"id": "123"}));

        let upgraded = registry.upgrade(event).expect("should upgrade");

        assert_eq!(upgraded.version(), 2);
    }

    #[test]
    fn registry_returns_error_for_unknown_type() {
        let registry = UpgraderRegistry::new();
        let event = VersionedEvent::new(1, "UnknownEvent", json!({}));

        let result = registry.upgrade(event);

        assert!(matches!(
            result,
            Err(SchemaUpgradeError::UnknownEventType(ref t)) if t == "UnknownEvent"
        ));
    }
}
