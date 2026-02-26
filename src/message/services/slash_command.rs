//! Slash-command orchestration service.

use minijinja::Environment;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::message::domain::{
    PlannedToolCall, SlashCommandError, SlashCommandExecution, SlashCommandExpansion,
    SlashCommandInvocation, ToolCallAudit, ToolCallStatus,
};
use crate::message::ports::slash_command::SlashCommandRegistry;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Service that executes slash commands using a registry.
#[derive(Clone)]
pub struct SlashCommandService<R>
where
    R: SlashCommandRegistry,
{
    registry: Arc<R>,
}

impl<R> SlashCommandService<R>
where
    R: SlashCommandRegistry,
{
    /// Creates a new slash-command service.
    #[must_use]
    pub const fn new(registry: Arc<R>) -> Self {
        Self { registry }
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
            .map_err(|error| SlashCommandError::Registry(error.to_string()))?
            .ok_or_else(|| SlashCommandError::UnknownCommand(invocation.command().to_owned()))?;

        let validated_parameters = definition.validate_parameters(invocation.parameters())?;
        let expanded_content = render_template(
            invocation.command(),
            &definition.expansion_template,
            &validated_parameters,
        )?;

        let planned_tool_calls = Self::plan_tool_calls(
            invocation.command(),
            &definition.tool_calls,
            &validated_parameters,
        )?;

        let tool_call_audits = planned_tool_calls
            .iter()
            .map(|call| ToolCallAudit::new(&call.call_id, &call.tool_name, ToolCallStatus::Queued))
            .collect();

        let expansion = build_expansion(
            invocation.command(),
            &validated_parameters,
            expanded_content,
        );
        Ok(SlashCommandExecution::new(
            invocation,
            expansion,
            planned_tool_calls,
            tool_call_audits,
        ))
    }

    fn plan_tool_calls(
        command: &str,
        templates: &[crate::message::domain::ToolCallTemplate],
        parameters: &BTreeMap<String, Value>,
    ) -> Result<Vec<PlannedToolCall>, SlashCommandError> {
        templates
            .iter()
            .enumerate()
            .map(|(index, template)| {
                let rendered_arguments =
                    render_template(command, &template.arguments_template, parameters)?;
                let arguments: Value =
                    serde_json::from_str(&rendered_arguments).map_err(|error| {
                        SlashCommandError::InvalidToolArgumentsTemplate {
                            tool_name: template.tool_name.clone(),
                            reason: error.to_string(),
                        }
                    })?;

                let call_id = build_deterministic_call_id(&DeterministicCallIdInput {
                    command,
                    index,
                    tool_name: &template.tool_name,
                    parameters,
                    arguments: &arguments,
                })?;

                Ok(PlannedToolCall::new(
                    call_id,
                    template.tool_name.clone(),
                    arguments,
                ))
            })
            .collect()
    }
}

fn render_template(
    command: &str,
    template: &str,
    parameters: &BTreeMap<String, Value>,
) -> Result<String, SlashCommandError> {
    let environment = Environment::new();
    let context = build_template_context(command, parameters);
    environment
        .render_str(template, context)
        .map_err(|error| SlashCommandError::TemplateRender {
            command: command.to_owned(),
            reason: error.to_string(),
        })
}

fn build_template_context(
    command: &str,
    parameters: &BTreeMap<String, Value>,
) -> Map<String, Value> {
    let mut context = Map::new();
    context.insert("command".to_owned(), Value::String(command.to_owned()));
    for (key, value) in parameters {
        context.insert(key.clone(), value.clone());
    }
    context
}

fn build_expansion(
    command: &str,
    parameters: &BTreeMap<String, Value>,
    expanded_content: String,
) -> SlashCommandExpansion {
    let mut expansion = SlashCommandExpansion::new(format!("/{command}"), expanded_content);
    for (key, value) in parameters {
        expansion = expansion.with_parameter(key.clone(), value.clone());
    }
    expansion
}

fn build_deterministic_call_id(
    input: &DeterministicCallIdInput<'_>,
) -> Result<String, SlashCommandError> {
    let mut payload = Map::new();
    payload.insert(
        "command".to_owned(),
        Value::String(input.command.to_owned()),
    );
    payload.insert("index".to_owned(), Value::String(input.index.to_string()));
    payload.insert(
        "tool_name".to_owned(),
        Value::String(input.tool_name.to_owned()),
    );
    payload.insert(
        "parameters".to_owned(),
        serde_json::to_value(input.parameters).map_err(|error| {
            SlashCommandError::TemplateRender {
                command: input.command.to_owned(),
                reason: error.to_string(),
            }
        })?,
    );
    payload.insert("arguments".to_owned(), input.arguments.clone());

    let canonical =
        serde_json::to_string(&payload).map_err(|error| SlashCommandError::TemplateRender {
            command: input.command.to_owned(),
            reason: error.to_string(),
        })?;

    let hash = fnv1a_hash(&canonical);
    Ok(format!("sc-{}-{hash:016x}", input.index))
}

fn fnv1a_hash(input: &str) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in input.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

struct DeterministicCallIdInput<'a> {
    command: &'a str,
    index: usize,
    tool_name: &'a str,
    parameters: &'a BTreeMap<String, Value>,
    arguments: &'a Value,
}
