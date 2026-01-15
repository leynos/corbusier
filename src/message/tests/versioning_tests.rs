//! Unit tests for schema versioning.

use crate::message::{
    error::SchemaUpgradeError,
    versioning::{EventUpgrader, MessageCreatedUpgrader, UpgraderRegistry, VersionedEvent},
};
use chrono::Utc;
use rstest::rstest;
use serde_json::json;

// ============================================================================
// VersionedEvent tests
// ============================================================================

#[rstest]
fn versioned_event_new() {
    let event = VersionedEvent::new(1, "TestEvent", json!({"key": "value"}));
    assert_eq!(event.version(), 1);
    assert_eq!(event.event_type(), "TestEvent");
    assert_eq!(event.data().get("key"), Some(&json!("value")));
}

#[rstest]
fn versioned_event_metadata_has_timestamp() {
    let before = Utc::now();
    let event = VersionedEvent::new(1, "TestEvent", json!({}));
    let after = Utc::now();

    // Verify timestamp is within the expected range
    let occurred_at = event.metadata().occurred_at;
    assert!(
        occurred_at >= before && occurred_at <= after,
        "occurred_at {occurred_at} should be between {before} and {after}"
    );
}

#[rstest]
fn versioned_event_set_version() {
    let mut event = VersionedEvent::new(1, "TestEvent", json!({}));
    event.set_version(2);
    assert_eq!(event.version(), 2);
}

#[rstest]
fn versioned_event_data_mut() {
    let mut event = VersionedEvent::new(1, "TestEvent", json!({"count": 1}));
    if let Some(obj) = event.data_mut().as_object_mut() {
        obj.insert("new_field".to_owned(), json!("added"));
    }
    assert!(event.data().get("new_field").is_some());
}

// ============================================================================
// MessageCreatedUpgrader tests
// ============================================================================

#[rstest]
#[case(0, false)]
#[case(1, true)]
#[case(2, true)]
#[case(3, false)]
fn upgrader_version_support(#[case] version: u32, #[case] expected: bool) {
    let upgrader = MessageCreatedUpgrader::new();
    assert_eq!(upgrader.supports_version(version), expected);
    // Current version is always 2
    assert_eq!(upgrader.current_version(), 2);
}

#[rstest]
fn upgrade_v1_to_v2_adds_metadata() {
    let upgrader = MessageCreatedUpgrader::new();
    let event = VersionedEvent::new(
        1,
        "MessageCreated",
        json!({
            "id": "msg-123",
            "content": [{"type": "text", "text": "Hello"}]
        }),
    );

    let upgraded = upgrader.upgrade(event).expect("should upgrade");

    assert_eq!(upgraded.version(), 2);
    assert!(upgraded.data().get("metadata").is_some());
    // Original fields preserved
    assert_eq!(upgraded.data().get("id"), Some(&json!("msg-123")));
}

#[rstest]
fn upgrade_v1_preserves_existing_metadata() {
    let upgrader = MessageCreatedUpgrader::new();
    // V1 event that somehow already has metadata
    let event = VersionedEvent::new(
        1,
        "MessageCreated",
        json!({
            "id": "msg-123",
            "metadata": {"agent": "test"}
        }),
    );

    let upgraded = upgrader.upgrade(event).expect("should upgrade");

    assert_eq!(upgraded.version(), 2);
    // Existing metadata preserved
    assert_eq!(
        upgraded.data().get("metadata"),
        Some(&json!({"agent": "test"}))
    );
}

#[rstest]
fn upgrade_v2_unchanged() {
    let upgrader = MessageCreatedUpgrader::new();
    let event = VersionedEvent::new(
        2,
        "MessageCreated",
        json!({
            "id": "msg-123",
            "metadata": {"key": "value"}
        }),
    );

    let upgraded = upgrader.upgrade(event).expect("should not modify v2");

    assert_eq!(upgraded.version(), 2);
    assert_eq!(
        upgraded.data().get("metadata"),
        Some(&json!({"key": "value"}))
    );
}

#[rstest]
fn upgrade_unsupported_version_fails() {
    let upgrader = MessageCreatedUpgrader::new();
    let event = VersionedEvent::new(99, "MessageCreated", json!({}));

    let result = upgrader.upgrade(event);

    assert!(matches!(
        result,
        Err(SchemaUpgradeError::UnsupportedVersion(99))
    ));
}

#[rstest]
fn upgrade_malformed_data_fails() {
    let upgrader = MessageCreatedUpgrader::new();
    // Data is an array, not an object
    let event = VersionedEvent::new(1, "MessageCreated", json!(["not", "an", "object"]));

    let result = upgrader.upgrade(event);

    assert!(matches!(result, Err(SchemaUpgradeError::MalformedData(_))));
}

// ============================================================================
// UpgraderRegistry tests
// ============================================================================

#[rstest]
fn registry_new_has_default_upgraders() {
    let registry = UpgraderRegistry::new();
    assert!(registry.has_upgrader("MessageCreated"));
}

#[rstest]
fn registry_empty_has_no_upgraders() {
    let registry = UpgraderRegistry::empty();
    assert!(!registry.has_upgrader("MessageCreated"));
}

#[rstest]
fn registry_register_custom_upgrader() {
    let mut registry = UpgraderRegistry::empty();
    registry.register("CustomEvent", Box::new(MessageCreatedUpgrader::new()));
    assert!(registry.has_upgrader("CustomEvent"));
}

#[rstest]
fn registry_upgrade_dispatches_correctly() {
    let registry = UpgraderRegistry::new();
    let event = VersionedEvent::new(1, "MessageCreated", json!({"id": "test"}));

    let upgraded = registry.upgrade(event).expect("should upgrade");

    assert_eq!(upgraded.version(), 2);
}

#[rstest]
fn registry_upgrade_unknown_type_fails() {
    let registry = UpgraderRegistry::new();
    let event = VersionedEvent::new(1, "UnknownEventType", json!({}));

    let result = registry.upgrade(event);

    assert!(matches!(
        result,
        Err(SchemaUpgradeError::UnknownEventType(ref t)) if t == "UnknownEventType"
    ));
}

#[rstest]
fn registry_current_version() {
    let registry = UpgraderRegistry::new();
    assert_eq!(registry.current_version("MessageCreated"), Some(2));
    assert_eq!(registry.current_version("Unknown"), None);
}

// ============================================================================
// Error type tests
// ============================================================================

#[rstest]
fn schema_upgrade_error_display() {
    let unsupported = SchemaUpgradeError::UnsupportedVersion(5);
    assert!(unsupported.to_string().contains('5'));

    let upgrade_failed = SchemaUpgradeError::upgrade_failed(1, 2, "something went wrong");
    assert!(upgrade_failed.to_string().contains('1'));
    assert!(upgrade_failed.to_string().contains('2'));

    let malformed = SchemaUpgradeError::malformed("bad data");
    assert!(malformed.to_string().contains("bad data"));
}
