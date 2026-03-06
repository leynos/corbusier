//! Unit tests for cross-cutting context and identity types.

use rstest::rstest;
use uuid::Uuid;

use super::RequestContext;
use super::ids::{CausationId, CorrelationId, SessionId, TenantId, UserId};

// ── ID newtype tests ─────────────────────────────────────────────

macro_rules! test_id_newtype {
    ($ty:ident, $mod_name:ident) => {
        mod $mod_name {
            use super::*;

            #[rstest]
            fn new_generates_unique_ids() {
                let a = $ty::new();
                let b = $ty::new();
                assert_ne!(a, b);
            }

            #[rstest]
            fn from_uuid_round_trips() {
                let uuid = Uuid::new_v4();
                let id = $ty::from_uuid(uuid);
                assert_eq!(id.into_inner(), uuid);
            }

            #[rstest]
            fn as_ref_returns_inner_uuid() {
                let uuid = Uuid::new_v4();
                let id = $ty::from_uuid(uuid);
                assert_eq!(id.as_ref(), &uuid);
            }

            #[rstest]
            fn display_matches_inner_uuid() {
                let uuid = Uuid::new_v4();
                let id = $ty::from_uuid(uuid);
                assert_eq!(id.to_string(), uuid.to_string());
            }

            #[rstest]
            fn default_creates_valid_id() {
                let id = $ty::default();
                // Default should produce a valid v4 UUID.
                assert!(!id.into_inner().is_nil());
            }

            #[rstest]
            fn clone_produces_equal_value() {
                let id = $ty::new();
                let cloned = id.clone();
                assert_eq!(id, cloned);
            }

            #[rstest]
            fn serde_round_trip() {
                let id = $ty::new();
                let json = serde_json::to_string(&id).expect("serialize");
                let deserialized: $ty = serde_json::from_str(&json).expect("deserialize");
                assert_eq!(id, deserialized);
            }
        }
    };
}

test_id_newtype!(TenantId, tenant_id_tests);
test_id_newtype!(CorrelationId, correlation_id_tests);
test_id_newtype!(CausationId, causation_id_tests);
test_id_newtype!(UserId, user_id_tests);
test_id_newtype!(SessionId, session_id_tests);

// ── RequestContext tests ─────────────────────────────────────────

#[rstest]
fn request_context_new_sets_required_fields() {
    let tenant_id = TenantId::new();
    let correlation_id = CorrelationId::new();
    let user_id = UserId::new();
    let session_id = SessionId::new();

    let ctx = RequestContext::new(tenant_id, correlation_id, user_id, session_id);

    assert_eq!(ctx.tenant_id(), tenant_id);
    assert_eq!(ctx.correlation_id(), correlation_id);
    assert_eq!(ctx.user_id(), user_id);
    assert_eq!(ctx.session_id(), session_id);
    assert!(ctx.causation_id().is_none());
}

#[rstest]
fn request_context_with_causation_id_sets_field() {
    let causation_id = CausationId::new();

    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    )
    .with_causation_id(causation_id);

    assert_eq!(ctx.causation_id(), Some(causation_id));
}

#[rstest]
fn request_context_clone_produces_equal_value() {
    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    )
    .with_causation_id(CausationId::new());

    let cloned = ctx.clone();
    assert_eq!(ctx, cloned);
}

#[rstest]
fn request_context_equality_requires_matching_fields() {
    let tenant_id = TenantId::new();
    let correlation_id = CorrelationId::new();
    let user_id = UserId::new();
    let session_id = SessionId::new();

    let ctx_a = RequestContext::new(tenant_id, correlation_id, user_id, session_id);
    let ctx_b = RequestContext::new(tenant_id, correlation_id, user_id, session_id);
    assert_eq!(ctx_a, ctx_b);

    // Different tenant_id should make them unequal.
    let ctx_c = RequestContext::new(TenantId::new(), correlation_id, user_id, session_id);
    assert_ne!(ctx_a, ctx_c);
}
