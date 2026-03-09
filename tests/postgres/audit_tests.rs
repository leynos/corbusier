//! Audit context tests for `PostgreSQL` message repository.
//!
//! Tests `store_with_audit` session variable propagation and `audit_logs` table
//! verification via the database trigger.

#![expect(
    clippy::too_many_arguments,
    reason = "rstest fixture injection with #[case] parameters requires multiple arguments"
)]

use crate::postgres::helpers::{
    BoxError, PostgresCluster, clock, ensure_template, fetch_audit_log_for_message,
    insert_conversation, postgres_cluster, setup_repository, test_request_context,
};
use corbusier::context::{CausationId, RequestContext};
use corbusier::message::{
    domain::{ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart},
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::rstest;

/// Tests `store_with_audit` correctly propagates audit context to `audit_logs` table.
///
/// Parameterized across two scenarios:
/// - With causation: all context fields propagated
/// - Without causation: `causation_id` absent from audit log
#[rstest]
#[case::with_causation(true, "with_causation")]
#[case::without_causation(false, "without_causation")]
#[tokio::test]
async fn store_with_audit_captures_context(
    clock: DefaultClock,
    test_request_context: RequestContext,
    postgres_cluster: Result<PostgresCluster, BoxError>,
    #[case] include_causation: bool,
    #[case] scenario: &str,
) -> Result<(), BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    let (temp_db, repo) = setup_repository(cluster).await?;

    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id).await?;

    let message = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Audited message"))],
        SequenceNumber::new(1),
        &clock,
    )?;

    let mut ctx = test_request_context;
    if include_causation {
        ctx = ctx.with_causation_id(CausationId::new());
    }

    repo.store_with_audit(&ctx, &message).await?;

    let retrieved = repo
        .find_by_id(&ctx, message.id())
        .await?
        .expect("message should exist");

    assert_eq!(retrieved.id(), message.id());

    // Verify audit_logs entry was created with correct context
    let audit_log = fetch_audit_log_for_message(cluster, temp_db.name(), message.id().into_inner())
        .await?
        .expect("audit log entry should exist");

    assert_eq!(audit_log.table_name, "messages");
    assert_eq!(audit_log.operation, "INSERT");
    assert_eq!(audit_log.row_id, Some(message.id().into_inner()));
    assert_eq!(
        audit_log.correlation_id,
        Some(ctx.correlation_id().into_inner()),
        "scenario: {scenario}"
    );
    assert_eq!(
        audit_log.causation_id,
        ctx.causation_id().map(CausationId::into_inner),
        "scenario: {scenario}"
    );
    assert_eq!(
        audit_log.user_id,
        Some(ctx.user_id().into_inner()),
        "scenario: {scenario}"
    );
    assert_eq!(
        audit_log.session_id,
        Some(ctx.session_id().into_inner()),
        "scenario: {scenario}"
    );
    Ok(())
}
