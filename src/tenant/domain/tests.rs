//! Unit tests for tenant domain primitives.

use mockable::DefaultClock;
use rstest::rstest;

use crate::context::UserId;

use super::{
    ParseTenantStatusError, Tenant, TenantDomainError, TenantId, TenantSlug, TenantStatus,
};

// ── TenantSlug validation ────────────────────────────────────────

#[rstest]
#[case("acme-corp")]
#[case("a")]
#[case("my-tenant-123")]
#[case("tenant1")]
fn valid_slugs_are_accepted(#[case] input: &str) {
    let slug = TenantSlug::new(input);
    assert!(slug.is_ok(), "expected '{input}' to be a valid slug");
    assert_eq!(slug.expect("valid slug").as_str(), input);
}

#[rstest]
fn slug_is_trimmed_and_lowercased() {
    let slug = TenantSlug::new("  Acme-Corp  ").expect("should accept after trim+lowercase");
    assert_eq!(slug.as_str(), "acme-corp");
}

#[rstest]
#[case("")]
#[case("   ")]
fn empty_or_whitespace_slug_is_rejected(#[case] input: &str) {
    let result = TenantSlug::new(input);
    assert!(matches!(result, Err(TenantDomainError::EmptySlug)));
}

#[rstest]
#[case("acme_corp")]
#[case("acme.corp")]
#[case("acme/corp")]
#[case("acme corp")]
#[case("acme@corp")]
fn invalid_characters_in_slug_rejected(#[case] input: &str) {
    let result = TenantSlug::new(input);
    assert!(
        matches!(result, Err(TenantDomainError::InvalidSlug(_))),
        "expected '{input}' to be rejected for invalid chars"
    );
}

#[rstest]
#[case("-acme")]
#[case("acme-")]
#[case("-")]
fn boundary_hyphen_slug_rejected(#[case] input: &str) {
    let result = TenantSlug::new(input);
    assert!(matches!(
        result,
        Err(TenantDomainError::SlugBoundaryHyphen(_))
    ));
}

#[rstest]
fn consecutive_hyphens_slug_rejected() {
    let result = TenantSlug::new("acme--corp");
    assert!(matches!(
        result,
        Err(TenantDomainError::SlugConsecutiveHyphens(_))
    ));
}

#[rstest]
#[case(63, true)]
#[case(64, false)]
fn slug_length_boundary(#[case] length: usize, #[case] expected_ok: bool) {
    let slug_str = "a".repeat(length);
    let result = TenantSlug::new(&slug_str);
    if expected_ok {
        assert!(result.is_ok(), "expected length {length} to be accepted");
    } else {
        assert!(
            matches!(result, Err(TenantDomainError::SlugTooLong(_))),
            "expected length {length} to be rejected"
        );
    }
}

#[rstest]
fn slug_display_matches_value() {
    let slug = TenantSlug::new("my-tenant").expect("valid slug");
    assert_eq!(slug.to_string(), "my-tenant");
}

#[rstest]
fn slug_as_ref_returns_str() {
    let slug = TenantSlug::new("my-tenant").expect("valid slug");
    let s: &str = slug.as_ref();
    assert_eq!(s, "my-tenant");
}

#[rstest]
fn slug_serde_round_trip() {
    let slug = TenantSlug::new("acme-corp").expect("valid slug");
    let json = serde_json::to_string(&slug).expect("serialize");
    let deserialized: TenantSlug = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(slug, deserialized);
}

// ── TenantStatus round-trip ──────────────────────────────────────

#[rstest]
#[case(TenantStatus::Active, "active")]
#[case(TenantStatus::Suspended, "suspended")]
fn tenant_status_as_str_round_trip(#[case] status: TenantStatus, #[case] expected: &str) {
    assert_eq!(status.as_str(), expected);
    let parsed = TenantStatus::try_from(expected).expect("should parse");
    assert_eq!(parsed, status);
}

#[rstest]
fn tenant_status_display_matches_as_str() {
    assert_eq!(TenantStatus::Active.to_string(), "active");
    assert_eq!(TenantStatus::Suspended.to_string(), "suspended");
}

#[rstest]
fn unknown_tenant_status_is_rejected() {
    let result = TenantStatus::try_from("unknown");
    assert!(matches!(result, Err(ParseTenantStatusError(_))));
}

#[rstest]
fn tenant_status_serde_round_trip() {
    let active = TenantStatus::Active;
    let json = serde_json::to_string(&active).expect("serialize");
    let deserialized: TenantStatus = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(active, deserialized);
}

// ── TenantId (re-exported from context) ──────────────────────────

#[rstest]
fn tenant_id_from_uuid_round_trips() {
    let uuid = uuid::Uuid::new_v4();
    let id = TenantId::from_uuid(uuid);
    assert_eq!(id.into_inner(), uuid);
}

// ── Tenant aggregate ─────────────────────────────────────────────

#[rstest]
fn new_tenant_defaults_to_active() {
    let clock = DefaultClock;
    let slug = TenantSlug::new("acme-corp").expect("valid slug");
    let owner = UserId::new();

    let tenant = Tenant::new(slug, "Acme Corporation", owner, &clock).expect("valid tenant");

    assert_eq!(tenant.slug().as_str(), "acme-corp");
    assert_eq!(tenant.display_name(), "Acme Corporation");
    assert_eq!(tenant.owner_user_id(), owner);
    assert_eq!(tenant.status(), TenantStatus::Active);
    assert_eq!(tenant.created_at(), tenant.updated_at());
}

#[rstest]
fn new_tenant_trims_display_name() {
    let clock = DefaultClock;
    let slug = TenantSlug::new("acme").expect("valid slug");
    let tenant = Tenant::new(slug, "  Acme Corp  ", UserId::new(), &clock).expect("valid tenant");
    assert_eq!(tenant.display_name(), "Acme Corp");
}

#[rstest]
fn new_tenant_with_empty_display_name_fails() {
    let clock = DefaultClock;
    let slug = TenantSlug::new("acme").expect("valid slug");
    let result = Tenant::new(slug, "   ", UserId::new(), &clock);
    assert!(matches!(result, Err(TenantDomainError::EmptyDisplayName)));
}

#[rstest]
fn suspend_changes_status_to_suspended() {
    let clock = DefaultClock;
    let slug = TenantSlug::new("acme").expect("valid slug");
    let mut tenant = Tenant::new(slug, "Acme Corp", UserId::new(), &clock).expect("valid tenant");
    tenant.suspend(&clock);
    assert_eq!(tenant.status(), TenantStatus::Suspended);
}

#[rstest]
fn reactivate_changes_status_to_active() {
    let clock = DefaultClock;
    let slug = TenantSlug::new("acme").expect("valid slug");
    let mut tenant = Tenant::new(slug, "Acme Corp", UserId::new(), &clock).expect("valid tenant");
    tenant.suspend(&clock);
    tenant.reactivate(&clock);
    assert_eq!(tenant.status(), TenantStatus::Active);
}

#[rstest]
fn tenant_owner_user_id_is_distinct_from_tenant_id() {
    let clock = DefaultClock;
    let slug = TenantSlug::new("acme").expect("valid slug");
    let owner = UserId::new();
    let tenant = Tenant::new(slug, "Acme Corp", owner, &clock).expect("valid tenant");

    // The owner user ID and tenant ID are distinct types with distinct values.
    assert_eq!(tenant.owner_user_id(), owner);
    assert_ne!(
        tenant.id().into_inner(),
        tenant.owner_user_id().into_inner()
    );
}

#[rstest]
fn tenant_from_persisted_preserves_all_fields() {
    let clock = DefaultClock;
    let slug = TenantSlug::new("acme").expect("valid slug");
    let owner = UserId::new();
    let original = Tenant::new(slug, "Acme Corp", owner, &clock).expect("valid tenant");

    let data = super::PersistedTenantData {
        id: original.id(),
        slug: original.slug().clone(),
        display_name: original.display_name().to_owned(),
        owner_user_id: original.owner_user_id(),
        status: original.status(),
        created_at: original.created_at(),
        updated_at: original.updated_at(),
    };
    let rehydrated = Tenant::from_persisted(data);

    assert_eq!(original, rehydrated);
}
