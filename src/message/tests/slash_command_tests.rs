//! Unit tests for slash-command parsing and execution.

use std::sync::Arc;

use rstest::{fixture, rstest};
use serde_json::json;

use crate::message::{
    adapters::memory::InMemorySlashCommandRegistry,
    domain::{
        CommandParameterSpec, CommandParameterType, SlashCommandDefinition, SlashCommandError,
        SlashCommandInvocation, ToolCallTemplate,
    },
    ports::slash_command::SlashCommandRegistry,
    services::SlashCommandService,
};

#[fixture]
fn slash_command_service() -> SlashCommandService<InMemorySlashCommandRegistry> {
    SlashCommandService::new(Arc::new(InMemorySlashCommandRegistry::new()))
}

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
fn registry_normalizes_custom_command_name_case() {
    let registry = InMemorySlashCommandRegistry::with_commands([SlashCommandDefinition::new(
        "Task",
        "Task command",
        "Task command",
    )])
    .expect("registry should accept valid command");

    let command = registry
        .find_by_name("task")
        .expect("lookup should succeed")
        .expect("command should exist");

    assert_eq!(command.command, "task");
}

#[rstest]
fn registry_rejects_invalid_parameter_schema_during_registration() {
    let invalid_definition = SlashCommandDefinition::new("task", "Task", "Task").with_parameter(
        CommandParameterSpec::new("action", CommandParameterType::Select, true),
    );

    let error = InMemorySlashCommandRegistry::with_commands([invalid_definition])
        .expect_err("registry should reject invalid select definition");

    assert!(matches!(
        error,
        crate::message::ports::slash_command::SlashCommandRegistryError::InvalidDefinition(_)
    ));
}

#[rstest]
fn parser_reports_unquoted_backslash_token_with_hint() {
    let error = SlashCommandInvocation::parse(r"/task action=start issue=C:\folder")
        .expect_err("unquoted backslashes should be rejected");

    assert_eq!(
        error,
        SlashCommandError::InvalidParameterToken {
            token: r"issue=C:\".to_owned(),
        }
    );
    assert!(
        error
            .to_string()
            .contains("quote values containing backslashes")
    );
}

#[rstest]
fn service_executes_deterministically_for_identical_input(
    slash_command_service: SlashCommandService<InMemorySlashCommandRegistry>,
) {
    let first = slash_command_service
        .execute("/task action=start issue=321")
        .expect("first execution should succeed");
    let second = slash_command_service
        .execute("/task action=start issue=321")
        .expect("second execution should succeed");

    assert_eq!(
        first.expansion().expanded_content,
        second.expansion().expanded_content
    );
    assert_eq!(first.planned_tool_calls(), second.planned_tool_calls());
    assert_eq!(first.tool_call_audits(), second.tool_call_audits());
}

#[rstest]
fn service_rejects_unknown_command(
    slash_command_service: SlashCommandService<InMemorySlashCommandRegistry>,
) {
    let error = slash_command_service
        .execute("/unknown action=start")
        .expect_err("unknown command should fail");

    assert_eq!(
        error,
        SlashCommandError::UnknownCommand("unknown".to_owned())
    );
}

#[rstest]
fn service_rejects_invalid_boolean_parameter(
    slash_command_service: SlashCommandService<InMemorySlashCommandRegistry>,
) {
    let error = slash_command_service
        .execute("/review action=sync include_summary=maybe")
        .expect_err("invalid bool should fail");

    assert_eq!(
        error,
        SlashCommandError::InvalidParameterValue {
            command: "review".to_owned(),
            parameter: "include_summary".to_owned(),
            reason: "expected true or false (case-insensitive)".to_owned(),
        }
    );
}

#[rstest]
fn service_accepts_mixed_case_boolean_parameter_values(
    slash_command_service: SlashCommandService<InMemorySlashCommandRegistry>,
) {
    slash_command_service
        .execute("/review action=sync include_summary=True")
        .expect("mixed-case true should be accepted");
    slash_command_service
        .execute("/review action=sync include_summary=FALSE")
        .expect("mixed-case false should be accepted");
}

#[rstest]
fn service_accepts_number_parameter_boundaries() {
    let definition =
        SlashCommandDefinition::new("set-count", "Set count", "Count {{ count }}").with_parameter(
            CommandParameterSpec::new("count", CommandParameterType::Number, true),
        );
    let registry =
        InMemorySlashCommandRegistry::with_commands([definition]).expect("registry should build");
    let service = SlashCommandService::new(Arc::new(registry));

    service
        .execute("/set-count count=-42")
        .expect("negative integer should parse");
    service
        .execute("/set-count count=9223372036854775807")
        .expect("maximum i64 integer should parse");
}

#[rstest]
#[case("1.0")]
#[case("1e3")]
fn service_rejects_non_integer_number_parameter_values(#[case] value: &str) {
    let definition =
        SlashCommandDefinition::new("set-count", "Set count", "Count {{ count }}").with_parameter(
            CommandParameterSpec::new("count", CommandParameterType::Number, true),
        );
    let registry =
        InMemorySlashCommandRegistry::with_commands([definition]).expect("registry should build");
    let service = SlashCommandService::new(Arc::new(registry));

    let error = service
        .execute(&format!("/set-count count={value}"))
        .expect_err("non-integer value should fail");

    assert_eq!(
        error,
        SlashCommandError::InvalidParameterValue {
            command: "set-count".to_owned(),
            parameter: "count".to_owned(),
            reason: "expected an integer number".to_owned(),
        }
    );
}

#[rstest]
fn service_accepts_case_insensitive_select_options(
    slash_command_service: SlashCommandService<InMemorySlashCommandRegistry>,
) {
    let execution = slash_command_service
        .execute("/task action=START issue=123")
        .expect("select options should be case-insensitive");

    assert_eq!(
        execution.expansion().parameters.get("action"),
        Some(&json!("start"))
    );
}

#[rstest]
fn service_accepts_quoted_string_with_json_sensitive_characters(
    slash_command_service: SlashCommandService<InMemorySlashCommandRegistry>,
) {
    let execution = slash_command_service
        .execute(r#"/task action=start issue="ENG \"123\" \\ path""#)
        .expect("quoted strings with escapes should render to valid JSON");

    let planned = execution
        .planned_tool_calls()
        .first()
        .expect("task command should produce one tool call");

    assert_eq!(planned.tool_name(), "task_service");
    assert_eq!(
        planned.arguments().get("issue"),
        Some(&json!(r#"ENG "123" \ path"#))
    );
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
