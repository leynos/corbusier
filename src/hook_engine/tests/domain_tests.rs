//! Domain tests for hook engine types.

use crate::hook_engine::domain::{
    ActionStatus, HookAction, HookActionId, HookActionType, HookDefinition, HookDomainError,
    HookExecutionStatus, HookId, HookPredicate, HookPriority, HookTriggerType,
};
use rstest::rstest;
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
