//! Tests for domain event adapter model types.
//!
//! Covers `DomainEventRow` and `NewDomainEvent` struct construction, field
//! preservation, and `Clone`/`Debug` implementations.

use crate::message::adapters::models::{DomainEventRow, NewDomainEvent};
use chrono::Utc;
use rstest::rstest;
use serde_json::json;
use uuid::Uuid;

// ============================================================================
// DomainEventRow Tests
// ============================================================================

#[rstest]
fn domain_event_row_struct_holds_all_fields() {
    let id = Uuid::new_v4();
    let aggregate_id = Uuid::new_v4();
    let aggregate_type = "Message".to_owned();
    let event_type = "MessageCreated".to_owned();
    let event_data = json!({"content": "test"});
    let event_version = 2;
    let occurred_at = Utc::now();
    let correlation_id = Some(Uuid::new_v4());
    let causation_id = Some(Uuid::new_v4());
    let user_id = Some(Uuid::new_v4());
    let session_id = Some(Uuid::new_v4());

    let row = DomainEventRow {
        id,
        aggregate_id,
        aggregate_type: aggregate_type.clone(),
        event_type: event_type.clone(),
        event_data: event_data.clone(),
        event_version,
        occurred_at,
        correlation_id,
        causation_id,
        user_id,
        session_id,
    };

    assert_eq!(row.id, id);
    assert_eq!(row.aggregate_id, aggregate_id);
    assert_eq!(row.aggregate_type, aggregate_type);
    assert_eq!(row.event_type, event_type);
    assert_eq!(row.event_data, event_data);
    assert_eq!(row.event_version, event_version);
    assert_eq!(row.occurred_at, occurred_at);
    assert_eq!(row.correlation_id, correlation_id);
    assert_eq!(row.causation_id, causation_id);
    assert_eq!(row.user_id, user_id);
    assert_eq!(row.session_id, session_id);
}

#[rstest]
fn domain_event_row_with_no_audit_fields() {
    let row = DomainEventRow {
        id: Uuid::new_v4(),
        aggregate_id: Uuid::new_v4(),
        aggregate_type: "Conversation".to_owned(),
        event_type: "ConversationStarted".to_owned(),
        event_data: json!({}),
        event_version: 1,
        occurred_at: Utc::now(),
        correlation_id: None,
        causation_id: None,
        user_id: None,
        session_id: None,
    };

    assert!(row.correlation_id.is_none());
    assert!(row.causation_id.is_none());
    assert!(row.user_id.is_none());
    assert!(row.session_id.is_none());
}

#[rstest]
fn domain_event_row_clone_preserves_all_fields() {
    let row = DomainEventRow {
        id: Uuid::new_v4(),
        aggregate_id: Uuid::new_v4(),
        aggregate_type: "Message".to_owned(),
        event_type: "MessageDeleted".to_owned(),
        event_data: json!({"reason": "spam"}),
        event_version: 1,
        occurred_at: Utc::now(),
        correlation_id: Some(Uuid::new_v4()),
        causation_id: None,
        user_id: Some(Uuid::new_v4()),
        session_id: None,
    };

    let cloned = row.clone();

    assert_eq!(cloned.id, row.id);
    assert_eq!(cloned.aggregate_id, row.aggregate_id);
    assert_eq!(cloned.aggregate_type, row.aggregate_type);
    assert_eq!(cloned.event_type, row.event_type);
    assert_eq!(cloned.event_data, row.event_data);
    assert_eq!(cloned.event_version, row.event_version);
    assert_eq!(cloned.occurred_at, row.occurred_at);
    assert_eq!(cloned.correlation_id, row.correlation_id);
    assert_eq!(cloned.causation_id, row.causation_id);
    assert_eq!(cloned.user_id, row.user_id);
    assert_eq!(cloned.session_id, row.session_id);
}

#[rstest]
fn domain_event_row_debug_format() {
    let row = DomainEventRow {
        id: Uuid::nil(),
        aggregate_id: Uuid::nil(),
        aggregate_type: "Test".to_owned(),
        event_type: "TestEvent".to_owned(),
        event_data: json!({}),
        event_version: 1,
        occurred_at: Utc::now(),
        correlation_id: None,
        causation_id: None,
        user_id: None,
        session_id: None,
    };

    let debug_str = format!("{row:?}");
    assert!(debug_str.contains("DomainEventRow"));
    assert!(debug_str.contains("TestEvent"));
}

// ============================================================================
// NewDomainEvent Tests
// ============================================================================

#[rstest]
fn new_domain_event_struct_holds_all_fields() {
    let id = Uuid::new_v4();
    let aggregate_id = Uuid::new_v4();
    let aggregate_type = "Message".to_owned();
    let event_type = "MessageCreated".to_owned();
    let event_data = json!({"content": "hello"});
    let event_version = 1;
    let occurred_at = Utc::now();
    let correlation_id = Some(Uuid::new_v4());
    let causation_id = Some(Uuid::new_v4());
    let user_id = Some(Uuid::new_v4());
    let session_id = Some(Uuid::new_v4());

    let event = NewDomainEvent {
        id,
        aggregate_id,
        aggregate_type: aggregate_type.clone(),
        event_type: event_type.clone(),
        event_data: event_data.clone(),
        event_version,
        occurred_at,
        correlation_id,
        causation_id,
        user_id,
        session_id,
    };

    assert_eq!(event.id, id);
    assert_eq!(event.aggregate_id, aggregate_id);
    assert_eq!(event.aggregate_type, aggregate_type);
    assert_eq!(event.event_type, event_type);
    assert_eq!(event.event_data, event_data);
    assert_eq!(event.event_version, event_version);
    assert_eq!(event.occurred_at, occurred_at);
    assert_eq!(event.correlation_id, correlation_id);
    assert_eq!(event.causation_id, causation_id);
    assert_eq!(event.user_id, user_id);
    assert_eq!(event.session_id, session_id);
}

#[rstest]
fn new_domain_event_with_minimal_fields() {
    let event = NewDomainEvent {
        id: Uuid::new_v4(),
        aggregate_id: Uuid::new_v4(),
        aggregate_type: "Conversation".to_owned(),
        event_type: "ConversationEnded".to_owned(),
        event_data: json!({}),
        event_version: 1,
        occurred_at: Utc::now(),
        correlation_id: None,
        causation_id: None,
        user_id: None,
        session_id: None,
    };

    assert!(event.correlation_id.is_none());
    assert!(event.causation_id.is_none());
    assert!(event.user_id.is_none());
    assert!(event.session_id.is_none());
    assert_eq!(event.event_version, 1);
}

#[rstest]
fn new_domain_event_clone_preserves_all_fields() {
    let event = NewDomainEvent {
        id: Uuid::new_v4(),
        aggregate_id: Uuid::new_v4(),
        aggregate_type: "Message".to_owned(),
        event_type: "MessageUpdated".to_owned(),
        event_data: json!({"old": "content", "new": "updated"}),
        event_version: 2,
        occurred_at: Utc::now(),
        correlation_id: Some(Uuid::new_v4()),
        causation_id: Some(Uuid::new_v4()),
        user_id: None,
        session_id: Some(Uuid::new_v4()),
    };

    let cloned = event.clone();

    assert_eq!(cloned.id, event.id);
    assert_eq!(cloned.aggregate_id, event.aggregate_id);
    assert_eq!(cloned.aggregate_type, event.aggregate_type);
    assert_eq!(cloned.event_type, event.event_type);
    assert_eq!(cloned.event_data, event.event_data);
    assert_eq!(cloned.event_version, event.event_version);
    assert_eq!(cloned.occurred_at, event.occurred_at);
    assert_eq!(cloned.correlation_id, event.correlation_id);
    assert_eq!(cloned.causation_id, event.causation_id);
    assert_eq!(cloned.user_id, event.user_id);
    assert_eq!(cloned.session_id, event.session_id);
}

#[rstest]
fn new_domain_event_debug_format() {
    let event = NewDomainEvent {
        id: Uuid::nil(),
        aggregate_id: Uuid::nil(),
        aggregate_type: "Test".to_owned(),
        event_type: "TestCreated".to_owned(),
        event_data: json!({}),
        event_version: 1,
        occurred_at: Utc::now(),
        correlation_id: None,
        causation_id: None,
        user_id: None,
        session_id: None,
    };

    let debug_str = format!("{event:?}");
    assert!(debug_str.contains("NewDomainEvent"));
    assert!(debug_str.contains("TestCreated"));
}
