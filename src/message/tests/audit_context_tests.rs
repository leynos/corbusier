//! Unit tests for `AuditContext` builder, `is_empty` semantics, and
//! `From<&RequestContext>` conversion.

use crate::context::{CausationId, CorrelationId, RequestContext, SessionId, TenantId, UserId};
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
    assert!(ctx.is_empty());
}

#[test]
fn audit_context_default_matches_empty() {
    let empty = AuditContext::empty();
    let default = AuditContext::default();

    assert_eq!(empty, default);
}

macro_rules! assert_single_field_builder {
    ($name:ident, $method:ident, $field:ident) => {
        #[test]
        fn $name() {
            let id = Uuid::new_v4();
            let ctx = AuditContext::empty().$method(id);
            assert_eq!(
                ctx,
                AuditContext {
                    $field: Some(id),
                    ..AuditContext::empty()
                }
            );
        }
    };
}

// ============================================================================
// with_* builder method tests
// ============================================================================

assert_single_field_builder!(
    audit_context_with_correlation_id_sets_field,
    with_correlation_id,
    correlation_id
);
assert_single_field_builder!(
    audit_context_with_causation_id_sets_field,
    with_causation_id,
    causation_id
);
assert_single_field_builder!(audit_context_with_user_id_sets_field, with_user_id, user_id);
assert_single_field_builder!(
    audit_context_with_session_id_sets_field,
    with_session_id,
    session_id
);

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

    let expected = AuditContext {
        correlation_id: Some(correlation),
        causation_id: Some(causation),
        user_id: Some(user),
        session_id: Some(session),
    };
    assert_eq!(ctx, expected);
}

#[test]
fn audit_context_builder_allows_partial_population() {
    let correlation = Uuid::new_v4();
    let user = Uuid::new_v4();

    let ctx = AuditContext::empty()
        .with_correlation_id(correlation)
        .with_user_id(user);

    assert_eq!(
        ctx,
        AuditContext {
            correlation_id: Some(correlation),
            user_id: Some(user),
            ..AuditContext::empty()
        }
    );
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

// ============================================================================
// From<&RequestContext> conversion tests
// ============================================================================

#[rstest]
#[case(Some(CausationId::new()))]
#[case(None)]
fn audit_context_from_request_context_maps_fields(#[case] causation_id: Option<CausationId>) {
    let correlation_id = CorrelationId::new();
    let user_id = UserId::new();
    let session_id = SessionId::new();

    let base = RequestContext::new(TenantId::new(), correlation_id, user_id, session_id);
    let mut expected = AuditContext::empty()
        .with_correlation_id(correlation_id.into_inner())
        .with_user_id(user_id.into_inner())
        .with_session_id(session_id.into_inner());

    let ctx = match causation_id {
        Some(cid) => {
            expected = expected.with_causation_id(cid.into_inner());
            base.with_causation_id(cid)
        }
        None => base,
    };

    assert_eq!(AuditContext::from(&ctx), expected);
}
