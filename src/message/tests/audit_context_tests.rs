//! Unit tests for `AuditContext` builder and `is_empty` semantics.

use crate::message::adapters::audit_context::AuditContext;
use rstest::rstest;
use uuid::Uuid;

// ============================================================================
// empty() tests
// ============================================================================

#[test]
fn audit_context_empty_has_all_none_fields() {
    let ctx = AuditContext::empty();

    assert!(ctx.correlation_id.is_none());
    assert!(ctx.causation_id.is_none());
    assert!(ctx.user_id.is_none());
    assert!(ctx.session_id.is_none());
}

#[test]
fn audit_context_default_matches_empty() {
    let empty = AuditContext::empty();
    let default = AuditContext::default();

    assert_eq!(empty, default);
}

// ============================================================================
// with_* builder method tests
// ============================================================================

#[test]
fn audit_context_with_correlation_id_sets_field() {
    let id = Uuid::new_v4();
    let ctx = AuditContext::empty().with_correlation_id(id);

    assert_eq!(ctx.correlation_id, Some(id));
    assert!(ctx.causation_id.is_none());
    assert!(ctx.user_id.is_none());
    assert!(ctx.session_id.is_none());
}

#[test]
fn audit_context_with_causation_id_sets_field() {
    let id = Uuid::new_v4();
    let ctx = AuditContext::empty().with_causation_id(id);

    assert!(ctx.correlation_id.is_none());
    assert_eq!(ctx.causation_id, Some(id));
    assert!(ctx.user_id.is_none());
    assert!(ctx.session_id.is_none());
}

#[test]
fn audit_context_with_user_id_sets_field() {
    let id = Uuid::new_v4();
    let ctx = AuditContext::empty().with_user_id(id);

    assert!(ctx.correlation_id.is_none());
    assert!(ctx.causation_id.is_none());
    assert_eq!(ctx.user_id, Some(id));
    assert!(ctx.session_id.is_none());
}

#[test]
fn audit_context_with_session_id_sets_field() {
    let id = Uuid::new_v4();
    let ctx = AuditContext::empty().with_session_id(id);

    assert!(ctx.correlation_id.is_none());
    assert!(ctx.causation_id.is_none());
    assert!(ctx.user_id.is_none());
    assert_eq!(ctx.session_id, Some(id));
}

#[test]
fn audit_context_builder_chain_preserves_all_fields() {
    let correlation = Uuid::new_v4();
    let causation = Uuid::new_v4();
    let user = Uuid::new_v4();
    let session = Uuid::new_v4();

    let ctx = AuditContext::empty()
        .with_correlation_id(correlation)
        .with_causation_id(causation)
        .with_user_id(user)
        .with_session_id(session);

    assert_eq!(ctx.correlation_id, Some(correlation));
    assert_eq!(ctx.causation_id, Some(causation));
    assert_eq!(ctx.user_id, Some(user));
    assert_eq!(ctx.session_id, Some(session));
}

#[test]
fn audit_context_builder_allows_partial_population() {
    let correlation = Uuid::new_v4();
    let user = Uuid::new_v4();

    let ctx = AuditContext::empty()
        .with_correlation_id(correlation)
        .with_user_id(user);

    assert_eq!(ctx.correlation_id, Some(correlation));
    assert!(ctx.causation_id.is_none());
    assert_eq!(ctx.user_id, Some(user));
    assert!(ctx.session_id.is_none());
}

// ============================================================================
// is_empty() tests
// ============================================================================

#[test]
fn audit_context_is_empty_true_when_all_none() {
    let ctx = AuditContext::empty();
    assert!(ctx.is_empty());
}

#[rstest]
#[case(AuditContext::empty().with_correlation_id(Uuid::new_v4()))]
#[case(AuditContext::empty().with_causation_id(Uuid::new_v4()))]
#[case(AuditContext::empty().with_user_id(Uuid::new_v4()))]
#[case(AuditContext::empty().with_session_id(Uuid::new_v4()))]
fn audit_context_is_empty_false_with_single_field(#[case] ctx: AuditContext) {
    assert!(!ctx.is_empty());
}

#[test]
fn audit_context_is_empty_false_when_all_populated() {
    let ctx = AuditContext::empty()
        .with_correlation_id(Uuid::new_v4())
        .with_causation_id(Uuid::new_v4())
        .with_user_id(Uuid::new_v4())
        .with_session_id(Uuid::new_v4());

    assert!(!ctx.is_empty());
}

// ============================================================================
// Clone and equality tests
// ============================================================================

#[test]
fn audit_context_clone_preserves_values() {
    let ctx = AuditContext::empty()
        .with_correlation_id(Uuid::new_v4())
        .with_user_id(Uuid::new_v4());

    let cloned = ctx.clone();

    assert_eq!(ctx, cloned);
}

#[test]
fn audit_context_equality_compares_all_fields() {
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();

    let ctx1 = AuditContext::empty().with_correlation_id(id1);
    let ctx2 = AuditContext::empty().with_correlation_id(id1);
    let ctx3 = AuditContext::empty().with_correlation_id(id2);

    assert_eq!(ctx1, ctx2);
    assert_ne!(ctx1, ctx3);
}

// ============================================================================
// Debug trait tests
// ============================================================================

#[test]
fn audit_context_debug_output() {
    let ctx = AuditContext::empty().with_correlation_id(Uuid::nil());

    let debug = format!("{ctx:?}");

    assert!(debug.contains("AuditContext"));
    assert!(debug.contains("correlation_id"));
}
