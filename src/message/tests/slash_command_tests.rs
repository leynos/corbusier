//! Unit tests for slash-command parsing and execution.

use std::sync::Arc;

use rstest::rstest;

use crate::message::{
    adapters::memory::InMemorySlashCommandRegistry,
    domain::{
        CommandParameterSpec, CommandParameterType, SlashCommandDefinition, SlashCommandError,
        SlashCommandInvocation, ToolCallTemplate,
    },
    services::SlashCommandService,
};

#[rstest]
fn parser_accepts_quoted_parameter_values() {
    let invocation = SlashCommandInvocation::parse("/task action=start issue=\"ENG 123\"")
        .expect("parser should accept quoted values");

    assert_eq!(invocation.command(), "task");
    assert_eq!(
        invocation.parameters().get("issue"),
        Some(&"ENG 123".to_owned())
    );
}

#[rstest]
fn parser_rejects_input_without_leading_slash() {
    let error =
        SlashCommandInvocation::parse("task action=start").expect_err("parser should reject");

    assert_eq!(error, SlashCommandError::MissingLeadingSlash);
}

#[rstest]
fn definition_rejects_invalid_select_value() {
    let definition = SlashCommandDefinition::new("task", "task", "task").with_parameter(
        CommandParameterSpec::new("action", CommandParameterType::Select, true)
            .with_options(["start", "create"]),
    );

    let invocation = SlashCommandInvocation::parse("/task action=delete")
        .expect("parser should accept structure");
    let error = definition
        .validate_parameters(invocation.parameters())
        .expect_err("validator should reject unknown option");

    assert!(matches!(
        error,
        SlashCommandError::InvalidParameterValue {
            parameter,
            command,
            ..
        } if parameter == "action" && command == "task"
    ));
}

#[rstest]
fn service_executes_deterministically_for_identical_input() {
    let service = SlashCommandService::new(Arc::new(InMemorySlashCommandRegistry::new()));

    let first = service
        .execute("/task action=start issue=321")
        .expect("first execution should succeed");
    let second = service
        .execute("/task action=start issue=321")
        .expect("second execution should succeed");

    assert_eq!(
        first.expansion.expanded_content,
        second.expansion.expanded_content
    );
    assert_eq!(first.planned_tool_calls, second.planned_tool_calls);
    assert_eq!(first.tool_call_audits, second.tool_call_audits);
}

#[rstest]
fn service_rejects_unknown_command() {
    let service = SlashCommandService::new(Arc::new(InMemorySlashCommandRegistry::new()));

    let error = service
        .execute("/unknown action=start")
        .expect_err("unknown command should fail");

    assert_eq!(
        error,
        SlashCommandError::UnknownCommand("unknown".to_owned())
    );
}

#[rstest]
fn service_rejects_invalid_boolean_parameter() {
    let service = SlashCommandService::new(Arc::new(InMemorySlashCommandRegistry::new()));

    let error = service
        .execute("/review action=sync include_summary=maybe")
        .expect_err("invalid bool should fail");

    assert!(matches!(
        error,
        SlashCommandError::InvalidParameterValue {
            parameter,
            command,
            ..
        } if parameter == "include_summary" && command == "review"
    ));
}

#[rstest]
fn service_rejects_tool_template_that_renders_invalid_json() {
    let registry = InMemorySlashCommandRegistry::with_commands([SlashCommandDefinition::new(
        "broken",
        "broken",
        "Broken {{ value }}",
    )
    .with_parameter(CommandParameterSpec::new(
        "value",
        CommandParameterType::String,
        true,
    ))
    .with_tool_call(ToolCallTemplate::new("broken_tool", "{not-json}"))])
    .expect("registry should build");

    let service = SlashCommandService::new(Arc::new(registry));
    let error = service
        .execute("/broken value=test")
        .expect_err("invalid JSON tool arguments should fail");

    assert!(matches!(
        error,
        SlashCommandError::InvalidToolArgumentsTemplate { tool_name, .. } if tool_name == "broken_tool"
    ));
}
