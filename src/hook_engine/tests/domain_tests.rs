//! Domain tests for hook engine types.

use crate::hook_engine::domain::{
    ActionResult, ActionResultDetails, ActionStatus, HookAction, HookActionId, HookActionType,
    HookDefinition, HookDomainError, HookExecutionInput, HookExecutionScope, HookExecutionStatus,
    HookId, HookPredicate, HookPriority, HookTriggerContext, HookTriggerType, PolicyAuditDecision,
    PolicyViolation, project_policy_audit_events,
};
use crate::message::domain::ConversationId;
use crate::task::domain::TaskId;
use chrono::Utc;
use rstest::rstest;
use serde_json::json;
use std::collections::HashSet;

#[test]
fn hook_id_rejects_empty() {
    let result = HookId::new("   ");
    assert_eq!(result, Err(HookDomainError::EmptyHookId));
}

#[test]
fn hook_action_id_rejects_empty() {
    let result = HookActionId::new("");
    assert_eq!(result, Err(HookDomainError::EmptyHookActionId));
}

#[test]
fn hook_definition_requires_actions() {
    let hook_id = HookId::new("hook-1").expect("valid hook id");
    let result = HookDefinition::new(hook_id, "Hook", HookTriggerType::TurnStart, Vec::new());
    assert_eq!(result, Err(HookDomainError::MissingActions));
}

#[rstest]
#[case(vec![ActionStatus::Succeeded], HookExecutionStatus::Succeeded)]
#[case(vec![ActionStatus::Skipped], HookExecutionStatus::Succeeded)]
#[case(vec![ActionStatus::Failed], HookExecutionStatus::Failed)]
#[case(
    vec![ActionStatus::Succeeded, ActionStatus::Failed],
    HookExecutionStatus::PartialFailure
)]
#[case(
    vec![ActionStatus::Failed, ActionStatus::Skipped],
    HookExecutionStatus::Failed
)]
fn hook_execution_status_aggregates(
    #[case] statuses: Vec<ActionStatus>,
    #[case] expected: HookExecutionStatus,
) {
    let result = HookExecutionStatus::from_action_statuses(&statuses);
    assert_eq!(result, expected);
}

#[test]
fn hook_trigger_type_all_includes_required_triggers() {
    let all: HashSet<_> = HookTriggerType::all().into_iter().collect();
    let required = [
        HookTriggerType::TurnStart,
        HookTriggerType::TurnEnd,
        HookTriggerType::PreToolUse,
        HookTriggerType::PostToolUse,
        HookTriggerType::PreCommit,
        HookTriggerType::PostCommit,
        HookTriggerType::PreMerge,
        HookTriggerType::PostMerge,
        HookTriggerType::PrePull,
        HookTriggerType::PostPull,
        HookTriggerType::PrePush,
        HookTriggerType::PostPush,
        HookTriggerType::PreDeploy,
        HookTriggerType::PostDeploy,
    ];

    for trigger in required {
        assert!(all.contains(&trigger));
    }
    assert_eq!(all.len(), required.len());
}

#[test]
fn hook_definition_accepts_predicate_and_priority() {
    let hook_id = HookId::new("hook-2").expect("valid hook id");
    let action_id = HookActionId::new("action-1").expect("valid action id");
    let action = HookAction::new(action_id, HookActionType::QualityGate);
    let predicate = HookPredicate::new(serde_json::json!({"key": "value"}));
    let definition = HookDefinition::new(hook_id, "Hook", HookTriggerType::PreCommit, vec![action])
        .expect("definition should be valid")
        .with_predicate(predicate)
        .with_priority(HookPriority::new(10));

    assert_eq!(definition.priority().value(), 10);
    assert_eq!(
        definition.predicate().data(),
        &serde_json::json!({"key": "value"})
    );
}

#[test]
fn hook_trigger_context_preserves_execution_scope() {
    let task_id = TaskId::new();
    let conversation_id = ConversationId::new();
    let scope = HookExecutionScope::default()
        .with_task_id(task_id)
        .with_conversation_id(conversation_id)
        .with_metadata(json!({"tool_name": "read_file"}));
    let context =
        HookTriggerContext::new_with_timestamp(HookTriggerType::PreToolUse, scope, Utc::now());

    assert_eq!(context.execution_scope().task_id(), Some(task_id));
    assert_eq!(
        context.execution_scope().conversation_id(),
        Some(conversation_id)
    );
    assert_eq!(context.metadata(), &json!({"tool_name": "read_file"}));
}

#[test]
fn project_policy_audit_events_extracts_allow_and_deny() {
    let task_id = TaskId::new();
    let conversation_id = ConversationId::new();
    let context = HookTriggerContext::new_with_timestamp(
        HookTriggerType::PreToolUse,
        HookExecutionScope::default()
            .with_task_id(task_id)
            .with_conversation_id(conversation_id)
            .with_metadata(json!({"tool_name": "read_file"})),
        Utc::now(),
    );
    let hook_id = HookId::new("hook-policy").expect("valid hook id");
    let execution = crate::hook_engine::domain::HookExecutionResult::new(HookExecutionInput {
        execution_id: crate::hook_engine::domain::HookExecutionId::new(),
        hook_id,
        trigger_context_id: context.id(),
        trigger_type: HookTriggerType::PreToolUse,
        predicate_data: serde_json::Value::Null,
        action_results: vec![
            ActionResult::new(ActionResultDetails {
                action_id: HookActionId::new("allow").expect("valid action id"),
                action_type: HookActionType::PolicyCheck,
                status: ActionStatus::Succeeded,
                output: json!({"decision": "allow"}),
                log_entries: Vec::new(),
            }),
            ActionResult::new(ActionResultDetails {
                action_id: HookActionId::new("deny").expect("valid action id"),
                action_type: HookActionType::PolicyCheck,
                status: ActionStatus::Succeeded,
                output: json!({
                    "decision": "deny",
                    "violation": {
                        "code": "tool.blocked",
                        "reason": "tool use is forbidden",
                    }
                }),
                log_entries: Vec::new(),
            }),
        ],
        executed_at: Utc::now(),
    });

    let events = project_policy_audit_events(&execution, &context)
        .expect("policy projection should succeed");

    assert_eq!(events.len(), 2);
    let allow_event = events.first().expect("expected allow policy event");
    let deny_event = events.get(1).expect("expected deny policy event");
    assert_eq!(allow_event.decision(), PolicyAuditDecision::Allow);
    assert_eq!(allow_event.task_id(), Some(task_id));
    assert_eq!(deny_event.decision(), PolicyAuditDecision::Deny);
    assert_eq!(
        deny_event
            .violation()
            .expect("deny event should include violation")
            .code(),
        "tool.blocked"
    );
}

#[test]
fn project_policy_audit_events_rejects_missing_decision() {
    let context = HookTriggerContext::new(HookTriggerType::PreToolUse, &mockable::DefaultClock);
    let hook_id = HookId::new("hook-policy-invalid").expect("valid hook id");
    let execution = crate::hook_engine::domain::HookExecutionResult::new(HookExecutionInput {
        execution_id: crate::hook_engine::domain::HookExecutionId::new(),
        hook_id,
        trigger_context_id: context.id(),
        trigger_type: HookTriggerType::PreToolUse,
        predicate_data: serde_json::Value::Null,
        action_results: vec![ActionResult::new(ActionResultDetails {
            action_id: HookActionId::new("invalid").expect("valid action id"),
            action_type: HookActionType::PolicyCheck,
            status: ActionStatus::Succeeded,
            output: json!({"status": "succeeded"}),
            log_entries: Vec::new(),
        })],
        executed_at: Utc::now(),
    });

    let error = project_policy_audit_events(&execution, &context)
        .expect_err("policy projection should fail");
    assert!(
        error
            .to_string()
            .contains("missing string field 'decision'"),
        "unexpected projection error: {error}"
    );
}

#[test]
fn policy_violation_rejects_blank_reason() {
    let error = PolicyViolation::new("tool.blocked", "   ")
        .expect_err("blank policy violation reason should fail");
    assert!(error.to_string().contains("violation reason"));
}
