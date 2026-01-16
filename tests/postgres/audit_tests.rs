//! Audit context tests for `PostgreSQL` message repository.
//!
//! Tests `store_with_audit` session variable propagation.

use crate::postgres::helpers::{
    CleanupGuard, clock, ensure_template, insert_conversation, setup_repository, test_runtime,
};
use corbusier::message::{
    adapters::audit_context::AuditContext,
    domain::{ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart},
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use pg_embedded_setup_unpriv::TestCluster;
use pg_embedded_setup_unpriv::test_support::shared_test_cluster;
use rstest::rstest;

#[rstest]
fn store_with_audit_sets_session_variables(
    clock: DefaultClock,
    shared_test_cluster: &'static TestCluster,
) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_audit_ctx_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let message = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Audited message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let audit = AuditContext::empty()
        .with_correlation_id(uuid::Uuid::new_v4())
        .with_causation_id(uuid::Uuid::new_v4())
        .with_user_id(uuid::Uuid::new_v4())
        .with_session_id(uuid::Uuid::new_v4());

    let rt = test_runtime();

    rt.block_on(repo.store_with_audit(&message, &audit))
        .expect("store_with_audit");

    let retrieved = rt
        .block_on(repo.find_by_id(message.id()))
        .expect("find")
        .expect("exists");

    assert_eq!(retrieved.id(), message.id());
}

#[rstest]
fn store_with_audit_handles_empty_context(
    clock: DefaultClock,
    shared_test_cluster: &'static TestCluster,
) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_audit_empty_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let message = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("msg");

    let audit = AuditContext::empty();
    assert!(audit.is_empty());

    let rt = test_runtime();

    rt.block_on(repo.store_with_audit(&message, &audit))
        .expect("store_with_audit with empty context");

    assert!(rt.block_on(repo.exists(message.id())).expect("exists"));
}

#[rstest]
fn store_with_audit_handles_partial_context(
    clock: DefaultClock,
    shared_test_cluster: &'static TestCluster,
) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_audit_partial_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let message = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("msg");

    let audit = AuditContext::empty().with_correlation_id(uuid::Uuid::new_v4());

    let rt = test_runtime();
    rt.block_on(repo.store_with_audit(&message, &audit))
        .expect("store_with_audit with partial context");

    assert!(rt.block_on(repo.exists(message.id())).expect("exists"));
}
