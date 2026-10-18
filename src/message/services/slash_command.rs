//! Slash-command orchestration service.

use minijinja::{Environment, Error as MiniJinjaError, ErrorKind as MiniJinjaErrorKind};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::hex::to_lower_hex_prefix;
use crate::message::domain::{
    PlannedToolCall, SlashCommandError, SlashCommandExecution, SlashCommandExpansion,
    SlashCommandInvocation, ToolCallAudit, ToolCallStatus,
};
use crate::message::ports::slash_command::{SlashCommandRegistry, SlashCommandRegistryError};

/// Service that executes slash commands using a registry.
#[derive(Clone)]
pub struct SlashCommandService<R>
where
    R: SlashCommandRegistry,
{
    registry: Arc<R>,
    environment: Arc<Environment<'static>>,
}

impl<R> SlashCommandService<R>
where
    R: SlashCommandRegistry,
{
    /// Creates a new slash-command service.
    #[must_use]
    pub fn new(registry: Arc<R>) -> Self {
        Self {
            registry,
            environment: Arc::new(create_template_environment()),
        }
    }

    /// Executes a raw slash-command input and returns deterministic output.
    ///
    /// # Errors
    ///
    /// Returns [`SlashCommandError`] when parsing, validation, template
    /// rendering, or registry lookup fails.
    pub fn execute(&self, raw_input: &str) -> Result<SlashCommandExecution, SlashCommandError> {
        let invocation = SlashCommandInvocation::parse(raw_input)?;
        let definition = self
            .registry
            .find_by_name(invocation.command())
            .map_err(map_registry_error)?
            .ok_or_else(|| SlashCommandError::UnknownCommand(invocation.command().to_owned()))?;

        let validated_parameters = definition.validate_parameters(invocation.parameters())?;
        let scope = CommandScope {
            command: invocation.command(),
            parameters: &validated_parameters,
        };
        let expanded_content = render_template(
            self.environment.as_ref(),
            scope,
            &definition.expansion_template,
        )?;

        let planned_tool_calls =
            Self::plan_tool_calls(self.environment.as_ref(), scope, &definition.tool_calls)?;

        let tool_call_audits = planned_tool_calls
            .iter()
            .map(|call| {
                ToolCallAudit::new(call.call_id(), call.tool_name(), ToolCallStatus::Queued)
            })
            .collect();

        let expansion = build_expansion(scope, expanded_content);
        Ok(SlashCommandExecution::new(
            invocation,
            expansion,
            planned_tool_calls,
            tool_call_audits,
        ))
    }

    /// Renders and parses every tool argument template in declaration order.
    ///
    /// The template position is part of each deterministic ID, so identical
    /// templates at positions zero and one still receive distinct IDs.
    ///
    /// # Errors
    ///
    /// Returns a template-rendering or invalid-arguments error when any
    /// template cannot be rendered or its output is not JSON.
    fn plan_tool_calls(
        environment: &Environment<'_>,
        scope: CommandScope<'_>,
        templates: &[crate::message::domain::ToolCallTemplate],
    ) -> Result<Vec<PlannedToolCall>, SlashCommandError> {
        templates
            .iter()
            .enumerate()
            .map(|(index, template)| {
                let rendered_arguments =
                    render_template(environment, scope, &template.arguments_template)?;
                let arguments: Value =
                    serde_json::from_str(&rendered_arguments).map_err(|error| {
                        SlashCommandError::InvalidToolArgumentsTemplate {
                            tool_name: template.tool_name.clone(),
                            reason: error.to_string(),
                        }
                    })?;

                let call_id = build_deterministic_call_id(&DeterministicCallIdInput {
                    command: scope.command,
                    index,
                    tool_name: &template.tool_name,
                    parameters: scope.parameters,
                    arguments: &arguments,
                });

                Ok(PlannedToolCall::new(
                    call_id,
                    template.tool_name.clone(),
                    arguments,
                ))
            })
            .collect()
    }
}

/// The command name and validated parameters that every rendering and planning
/// step of one execution reads together.
///
/// Scope: private to this module and borrowed for a single `execute` call, so
/// it adds no ownership or lifetime obligations beyond that call. It exists to
/// stop the two values being threaded through each helper as loose arguments.
#[derive(Clone, Copy)]
struct CommandScope<'a> {
    command: &'a str,
    parameters: &'a BTreeMap<String, Value>,
}

/// Renders a template with the command name and validated parameters.
///
/// For example, a `task` scope with `issue = 42` renders
/// `{{ command }} {{ issue }}` as `task 42`.
fn render_template(
    environment: &Environment<'_>,
    scope: CommandScope<'_>,
    template: &str,
) -> Result<String, SlashCommandError> {
    let context = build_template_context(scope);
    environment
        .render_str(template, context)
        .map_err(|error| SlashCommandError::TemplateRender {
            command: scope.command.to_owned(),
            reason: error.to_string(),
        })
}

/// Makes the command name and validated parameters available to templates.
///
/// A `task` scope with `issue = 42` produces the JSON context
/// `{"command":"task","issue":42}`.
fn build_template_context(scope: CommandScope<'_>) -> Map<String, Value> {
    let mut context = Map::new();
    context.insert(
        "command".to_owned(),
        Value::String(scope.command.to_owned()),
    );
    for (key, value) in scope.parameters {
        context.insert(key.clone(), value.clone());
    }
    context
}

/// Builds persisted expansion details from the command and rendered content.
///
/// A `task` scope produces `/task` and carries its validated parameters into
/// the expansion metadata.
fn build_expansion(scope: CommandScope<'_>, expanded_content: String) -> SlashCommandExpansion {
    let mut expansion = SlashCommandExpansion::new(format!("/{}", scope.command), expanded_content);
    for (key, value) in scope.parameters {
        expansion = expansion.with_parameter(key.clone(), value.clone());
    }
    expansion
}

/// Builds an indexed slash-command ID from the canonical call payload.
///
/// For `/task action=start issue=42`, the first planned call is
/// `sc-0-81e30d6fd777c32f`.
fn build_deterministic_call_id(input: &DeterministicCallIdInput<'_>) -> String {
    let canonical = canonical_call_id_payload(input);
    let digest = Sha256::digest(canonical.as_bytes());
    let hash_suffix = to_lower_hex_prefix(&digest, CALL_ID_DIGEST_BYTES);
    format!("sc-{}-{hash_suffix}", input.index)
}

/// Number of leading digest bytes rendered into a slash-command call ID.
///
/// Sixty-four bits of digest keeps identifiers short while leaving collisions
/// implausible for the number of tool calls a single command can plan.
const CALL_ID_DIGEST_BYTES: usize = 8;

/// Serialises ID inputs in a stable field order before hashing.
///
/// The payload includes the command, call index, tool, validated parameters,
/// and rendered arguments, so changing any of these changes the digest.
fn canonical_call_id_payload(input: &DeterministicCallIdInput<'_>) -> String {
    let mut canonical = String::new();
    canonical.push_str("command=");
    canonical.push_str(input.command);
    canonical.push_str(";index=");
    canonical.push_str(&input.index.to_string());
    canonical.push_str(";tool_name=");
    canonical.push_str(input.tool_name);
    canonical.push_str(";parameters=");
    for (key, value) in input.parameters {
        canonical.push_str(key);
        canonical.push('=');
        canonical.push_str(&value.to_string());
        canonical.push(';');
    }
    canonical.push_str(";arguments=");
    canonical.push_str(&input.arguments.to_string());
    canonical
}

struct DeterministicCallIdInput<'a> {
    command: &'a str,
    index: usize,
    tool_name: &'a str,
    parameters: &'a BTreeMap<String, Value>,
    arguments: &'a Value,
}

fn create_template_environment() -> Environment<'static> {
    let mut environment = Environment::new();
    environment.add_filter("json_string", json_string_filter);
    environment
}

fn json_string_filter(value: &str) -> Result<String, MiniJinjaError> {
    serde_json::to_string(&value).map_err(|error| {
        MiniJinjaError::new(
            MiniJinjaErrorKind::InvalidOperation,
            format!("failed to encode string as JSON: {error}"),
        )
    })
}

fn map_registry_error(error: SlashCommandRegistryError) -> SlashCommandError {
    match error {
        SlashCommandRegistryError::InvalidDefinition(source) => {
            SlashCommandError::RegistryInvalidDefinition { source }
        }
        SlashCommandRegistryError::Unavailable(source) => {
            SlashCommandError::RegistryUnavailable { source }
        }
    }
}
