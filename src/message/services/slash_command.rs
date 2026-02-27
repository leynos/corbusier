//! Slash-command orchestration service.

use minijinja::{Environment, Error as MiniJinjaError, ErrorKind as MiniJinjaErrorKind};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

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
        let expanded_content = render_template(
            self.environment.as_ref(),
            invocation.command(),
            &definition.expansion_template,
            &validated_parameters,
        )?;

        let planned_tool_calls = Self::plan_tool_calls(
            self.environment.as_ref(),
            invocation.command(),
            &definition.tool_calls,
            &validated_parameters,
        )?;

        let tool_call_audits = planned_tool_calls
            .iter()
            .map(|call| {
                ToolCallAudit::new(call.call_id(), call.tool_name(), ToolCallStatus::Queued)
            })
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
        environment: &Environment<'_>,
        command: &str,
        templates: &[crate::message::domain::ToolCallTemplate],
        parameters: &BTreeMap<String, Value>,
    ) -> Result<Vec<PlannedToolCall>, SlashCommandError> {
        templates
            .iter()
            .enumerate()
            .map(|(index, template)| {
                let rendered_arguments = render_template(
                    environment,
                    command,
                    &template.arguments_template,
                    parameters,
                )?;
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

fn render_template(
    environment: &Environment<'_>,
    command: &str,
    template: &str,
    parameters: &BTreeMap<String, Value>,
) -> Result<String, SlashCommandError> {
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

fn build_deterministic_call_id(input: &DeterministicCallIdInput<'_>) -> String {
    let canonical = canonical_call_id_payload(input);
    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    let hash = hasher.finish();
    format!("sc-{}-{hash:016x}", input.index)
}

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
        SlashCommandRegistryError::InvalidDefinition(reason) => {
            SlashCommandError::RegistryInvalidDefinition { reason }
        }
        SlashCommandRegistryError::Unavailable(reason) => {
            SlashCommandError::RegistryUnavailable { reason }
        }
    }
}
