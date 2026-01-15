//! Unit tests for domain identifier types.

use crate::message::domain::{ConversationId, MessageId, SequenceNumber, TurnId};
use rstest::rstest;

// ============================================================================
// MessageId tests
// ============================================================================

#[rstest]
fn message_id_new_creates_non_nil() {
    let id = MessageId::new();
    assert!(!id.as_ref().is_nil());
}

#[rstest]
fn message_id_default_creates_non_nil() {
    let id = MessageId::default();
    assert!(!id.as_ref().is_nil());
}

#[rstest]
fn message_id_different_ids_not_equal() {
    let id1 = MessageId::new();
    let id2 = MessageId::new();
    assert_ne!(id1, id2);
}

#[rstest]
fn message_id_from_uuid_preserves_value() {
    let uuid = uuid::Uuid::new_v4();
    let id = MessageId::from_uuid(uuid);
    assert_eq!(id.as_ref(), &uuid);
    assert_eq!(id.into_inner(), uuid);
}

#[rstest]
fn message_id_display() {
    let uuid =
        uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("valid UUID string");
    let id = MessageId::from_uuid(uuid);
    assert_eq!(id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
}

// ============================================================================
// ConversationId tests
// ============================================================================

#[rstest]
fn conversation_id_new_creates_non_nil() {
    let id = ConversationId::new();
    assert!(!id.as_ref().is_nil());
}

#[rstest]
fn conversation_id_default_creates_non_nil() {
    let id = ConversationId::default();
    assert!(!id.as_ref().is_nil());
}

#[rstest]
fn conversation_id_different_ids_not_equal() {
    let id1 = ConversationId::new();
    let id2 = ConversationId::new();
    assert_ne!(id1, id2);
}

#[rstest]
fn conversation_id_from_uuid_preserves_value() {
    let uuid = uuid::Uuid::new_v4();
    let id = ConversationId::from_uuid(uuid);
    assert_eq!(id.as_ref(), &uuid);
    assert_eq!(id.into_inner(), uuid);
}

// ============================================================================
// TurnId tests
// ============================================================================

#[rstest]
fn turn_id_new_creates_non_nil() {
    let id = TurnId::new();
    assert!(!id.as_ref().is_nil());
}

#[rstest]
fn turn_id_default_creates_non_nil() {
    let id = TurnId::default();
    assert!(!id.as_ref().is_nil());
}

#[rstest]
fn turn_id_different_ids_not_equal() {
    let id1 = TurnId::new();
    let id2 = TurnId::new();
    assert_ne!(id1, id2);
}

#[rstest]
fn turn_id_from_uuid_preserves_value() {
    let uuid = uuid::Uuid::new_v4();
    let id = TurnId::from_uuid(uuid);
    assert_eq!(id.as_ref(), &uuid);
    assert_eq!(id.into_inner(), uuid);
}

// ============================================================================
// SequenceNumber tests
// ============================================================================

#[rstest]
fn sequence_number_new_stores_value() {
    let seq = SequenceNumber::new(42);
    assert_eq!(seq.value(), 42);
}

#[rstest]
fn sequence_number_next_increments() {
    let seq = SequenceNumber::new(1);
    assert_eq!(seq.next().value(), 2);
}

#[rstest]
fn sequence_number_from_u64() {
    let seq: SequenceNumber = 100.into();
    assert_eq!(seq.value(), 100);
}

#[rstest]
fn sequence_number_ordering() {
    let seq1 = SequenceNumber::new(1);
    let seq2 = SequenceNumber::new(2);
    assert!(seq1 < seq2);
}

#[rstest]
fn sequence_number_next_saturates_at_max() {
    let seq = SequenceNumber::new(u64::MAX);
    // Verify saturation: calling next() on u64::MAX should still return u64::MAX
    assert_eq!(seq.next().value(), u64::MAX);
}
