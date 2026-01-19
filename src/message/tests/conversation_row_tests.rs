//! Tests for conversation-related adapter model types.
//!
//! Covers `NewConversation` and `ConversationRow` struct construction, field
//! preservation, and `Clone`/`Debug` implementations.

use crate::message::adapters::models::{ConversationRow, NewConversation};
use chrono::Utc;
use rstest::rstest;
use serde_json::json;
use uuid::Uuid;

// ============================================================================
// NewConversation Tests
// ============================================================================

#[rstest]
fn new_conversation_sets_default_state() {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let conv = NewConversation::new(id, now);

    assert_eq!(conv.id, id);
    assert_eq!(conv.state, "active");
    assert!(conv.task_id.is_none());
    assert_eq!(conv.created_at, now);
    assert_eq!(conv.updated_at, now);
}

#[rstest]
fn new_conversation_has_empty_context() {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let conv = NewConversation::new(id, now);

    assert!(conv.context.is_object());
    let context_obj = conv.context.as_object().expect("context should be object");
    assert!(context_obj.is_empty());
}

// ============================================================================
// ConversationRow Tests
// ============================================================================

#[rstest]
fn conversation_row_struct_holds_all_fields() {
    let id = Uuid::new_v4();
    let task_id = Some(Uuid::new_v4());
    let context = json!({"key": "value"});
    let state = "active".to_owned();
    let created_at = Utc::now();
    let updated_at = Utc::now();

    let row = ConversationRow {
        id,
        task_id,
        context: context.clone(),
        state: state.clone(),
        created_at,
        updated_at,
    };

    assert_eq!(row.id, id);
    assert_eq!(row.task_id, task_id);
    assert_eq!(row.context, context);
    assert_eq!(row.state, state);
    assert_eq!(row.created_at, created_at);
    assert_eq!(row.updated_at, updated_at);
}

#[rstest]
fn conversation_row_clone_preserves_fields() {
    let row = ConversationRow {
        id: Uuid::new_v4(),
        task_id: None,
        context: json!({}),
        state: "completed".to_owned(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let cloned = row.clone();

    assert_eq!(cloned.id, row.id);
    assert_eq!(cloned.task_id, row.task_id);
    assert_eq!(cloned.context, row.context);
    assert_eq!(cloned.state, row.state);
    assert_eq!(cloned.created_at, row.created_at);
    assert_eq!(cloned.updated_at, row.updated_at);
}

#[rstest]
fn conversation_row_debug_format() {
    let row = ConversationRow {
        id: Uuid::nil(),
        task_id: None,
        context: json!({}),
        state: "active".to_owned(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let debug_str = format!("{row:?}");
    assert!(debug_str.contains("ConversationRow"));
    assert!(debug_str.contains("active"));
}
