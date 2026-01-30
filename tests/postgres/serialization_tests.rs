//! Serialization round-trip tests for `PostgreSQL` message repository.
//!
//! Tests role parsing, JSONB content, metadata, and UUID handling.

use crate::postgres::helpers::{
    BoxError, PostgresCluster, RoleResult, clock, ensure_template, insert_conversation,
    postgres_cluster, setup_repository,
};
use corbusier::message::{
    domain::{
        AgentResponseAudit, AgentResponseStatus, AttachmentPart, ContentPart, ConversationId,
        Message, MessageId, MessageMetadata, Role, SequenceNumber, TextPart, ToolCallAudit,
        ToolCallPart, ToolCallStatus, ToolResultPart,
    },
    ports::repository::MessageRepository,
};
use diesel::prelude::*;
use mockable::DefaultClock;
use rstest::rstest;

// ============================================================================
// Role Round-Trip Tests
// ============================================================================

#[rstest]
#[case(Role::User, "user")]
#[case(Role::Assistant, "assistant")]
#[case(Role::Tool, "tool")]
#[case(Role::System, "system")]
#[tokio::test]
async fn role_round_trip_through_persistence(
    clock: DefaultClock,
    postgres_cluster: Result<PostgresCluster, BoxError>,
    #[case] role: Role,
    #[case] expected_str: &str,
) -> Result<(), BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    let (temp_db, repo) = setup_repository(cluster).await?;

    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id).await?;

    let message = Message::new(
        conv_id,
        role,
        vec![ContentPart::Text(TextPart::new("Role test"))],
        SequenceNumber::new(1),
        &clock,
    )?;

    repo.store(&message).await?;

    let url = cluster.connection().database_url(temp_db.name());
    let mut conn = PgConnection::establish(&url).map_err(|err| Box::new(err) as BoxError)?;
    let stored_role: String = diesel::sql_query("SELECT role FROM messages WHERE id = $1")
        .bind::<diesel::sql_types::Uuid, _>(message.id().into_inner())
        .get_result::<RoleResult>(&mut conn)
        .map_err(|err| Box::new(err) as BoxError)?
        .role;

    assert_eq!(stored_role, expected_str);
    drop(conn);

    let retrieved = repo
        .find_by_id(message.id())
        .await?
        .expect("message should exist");

    assert_eq!(retrieved.role(), role);
    Ok(())
}

// ============================================================================
// JSONB Round-Trip Tests
// ============================================================================

#[rstest]
#[tokio::test]
async fn content_jsonb_round_trip_with_multiple_parts(
    clock: DefaultClock,
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<(), BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    let (temp_db, repo) = setup_repository(cluster).await?;

    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id).await?;

    let content = vec![
        ContentPart::Text(TextPart::new("Hello world")),
        ContentPart::Attachment(AttachmentPart::new("image/png", "iVBORw0KGgo=")),
        ContentPart::ToolCall(ToolCallPart::new(
            "call_123",
            "search",
            serde_json::json!({"query": "test"}),
        )),
    ];

    let message = Message::new(
        conv_id,
        Role::Assistant,
        content,
        SequenceNumber::new(1),
        &clock,
    )?;

    repo.store(&message).await?;

    let retrieved = repo
        .find_by_id(message.id())
        .await?
        .expect("message should exist");

    let [first, second, third] = retrieved.content() else {
        panic!(
            "Expected 3 content parts, got {}",
            retrieved.content().len()
        );
    };

    match first {
        ContentPart::Text(text) => assert_eq!(text.text, "Hello world"),
        other => panic!("Expected Text, got {other:?}"),
    }

    match second {
        ContentPart::Attachment(att) => {
            assert_eq!(att.mime_type, "image/png");
            assert_eq!(att.data, "iVBORw0KGgo=");
        }
        other => panic!("Expected Attachment, got {other:?}"),
    }

    match third {
        ContentPart::ToolCall(call) => {
            assert_eq!(call.call_id, "call_123");
            assert_eq!(call.name, "search");
            assert_eq!(call.arguments, serde_json::json!({"query": "test"}));
        }
        other => panic!("Expected ToolCall, got {other:?}"),
    }
    Ok(())
}

#[rstest]
#[tokio::test]
async fn tool_result_jsonb_round_trip(
    clock: DefaultClock,
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<(), BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    let (temp_db, repo) = setup_repository(cluster).await?;

    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id).await?;

    let success_result =
        ToolResultPart::success("call_456", serde_json::json!({"result": "found 42 items"}));
    let failure_result = ToolResultPart::failure("call_789", "Network timeout");

    let message = Message::new(
        conv_id,
        Role::Tool,
        vec![
            ContentPart::ToolResult(success_result),
            ContentPart::ToolResult(failure_result),
        ],
        SequenceNumber::new(1),
        &clock,
    )?;

    repo.store(&message).await?;

    let retrieved = repo
        .find_by_id(message.id())
        .await?
        .expect("message should exist");

    let [first, second] = retrieved.content() else {
        panic!(
            "Expected 2 content parts, got {}",
            retrieved.content().len()
        );
    };

    match first {
        ContentPart::ToolResult(result) => {
            assert_eq!(result.call_id, "call_456");
            assert!(result.success);
            assert_eq!(
                result.content,
                serde_json::json!({"result": "found 42 items"})
            );
        }
        other => panic!("Expected ToolResult, got {other:?}"),
    }

    match second {
        ContentPart::ToolResult(result) => {
            assert_eq!(result.call_id, "call_789");
            assert!(!result.success);
        }
        other => panic!("Expected ToolResult, got {other:?}"),
    }
    Ok(())
}

#[rstest]
#[tokio::test]
async fn metadata_jsonb_round_trip(
    clock: DefaultClock,
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<(), BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    let (temp_db, repo) = setup_repository(cluster).await?;

    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id).await?;

    let metadata = MessageMetadata::with_agent_backend("claude-3-opus")
        .with_tool_call_audit(ToolCallAudit::new(
            "call-99",
            "read_file",
            ToolCallStatus::Succeeded,
        ))
        .with_agent_response_audit(
            AgentResponseAudit::new(AgentResponseStatus::Completed).with_response_id("resp-1"),
        );

    let message = Message::builder(conv_id, Role::Assistant, SequenceNumber::new(1))
        .with_content(ContentPart::Text(TextPart::new("Response")))
        .with_metadata(metadata)
        .build(&clock)?;

    repo.store(&message).await?;

    let retrieved = repo
        .find_by_id(message.id())
        .await?
        .expect("message should exist");

    assert_eq!(
        retrieved.metadata().agent_backend,
        Some("claude-3-opus".to_owned())
    );
    assert_eq!(retrieved.metadata().tool_call_audits.len(), 1);
    let tool_audit = retrieved
        .metadata()
        .tool_call_audits
        .first()
        .expect("tool call audit should exist");
    assert_eq!(tool_audit.status, ToolCallStatus::Succeeded);
    assert_eq!(
        retrieved
            .metadata()
            .agent_response_audit
            .as_ref()
            .expect("agent response audit should exist")
            .status,
        AgentResponseStatus::Completed
    );
    Ok(())
}

// ============================================================================
// Domain Invariant Tests
// ============================================================================

#[rstest]
#[tokio::test]
async fn from_persisted_preserves_all_domain_invariants(
    clock: DefaultClock,
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<(), BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    let (temp_db, repo) = setup_repository(cluster).await?;

    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id).await?;

    let original = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Test content"))],
        SequenceNumber::new(42),
        &clock,
    )?;

    repo.store(&original).await?;

    let retrieved = repo
        .find_by_id(original.id())
        .await?
        .expect("message should exist");

    assert!(
        !retrieved.id().into_inner().is_nil(),
        "ID should not be nil"
    );
    assert!(
        !retrieved.conversation_id().into_inner().is_nil(),
        "Conversation ID should not be nil"
    );
    assert!(
        !retrieved.content().is_empty(),
        "Content should not be empty"
    );
    assert_eq!(retrieved.sequence_number().value(), 42);

    // Tighter tolerance for local database operations
    let time_diff = (original.created_at() - retrieved.created_at())
        .num_milliseconds()
        .abs();
    assert!(
        time_diff < 100,
        "Timestamp should be preserved within 100ms, diff was {time_diff}ms"
    );
    Ok(())
}

// ============================================================================
// UUID Handling Tests
// ============================================================================

#[rstest]
#[tokio::test]
async fn uuid_round_trip_preserves_values(
    clock: DefaultClock,
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<(), BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    let (temp_db, repo) = setup_repository(cluster).await?;

    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id).await?;

    let specific_msg_id = MessageId::from_uuid(uuid::Uuid::parse_str(
        "550e8400-e29b-41d4-a716-446655440000",
    )?);

    let message = Message::builder(conv_id, Role::User, SequenceNumber::new(1))
        .with_id(specific_msg_id)
        .with_content(ContentPart::Text(TextPart::new("UUID test")))
        .build(&clock)?;

    repo.store(&message).await?;

    let retrieved = repo
        .find_by_id(specific_msg_id)
        .await?
        .expect("message should exist");

    assert_eq!(
        retrieved.id().into_inner().to_string(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
    assert_eq!(retrieved.conversation_id(), conv_id);
    Ok(())
}
