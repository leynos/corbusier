//! Slash-command definition and parameter validation.

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::collections::{BTreeMap, HashSet};

use super::SlashCommandError;

/// Parameter type for slash-command validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandParameterType {
    /// Free-form string value.
    String,
    /// Integer value.
    Number,
    /// Boolean value (`true` or `false`).
    Boolean,
    /// Enumeration with allowed options.
    Select,
}

/// Parameter specification for a slash command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandParameterSpec {
    /// Parameter name.
    pub name: String,
    /// Parameter type.
    pub parameter_type: CommandParameterType,
    /// Whether the parameter is required.
    pub required: bool,
    /// Allowed options for `select` parameters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
}

impl CommandParameterSpec {
    /// Creates a parameter specification.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        parameter_type: CommandParameterType,
        required: bool,
    ) -> Self {
        Self {
            name: name.into().to_ascii_lowercase(),
            parameter_type,
            required,
            options: Vec::new(),
        }
    }

    /// Adds allowed options for `select` parameters.
    #[must_use]
    pub fn with_options(mut self, options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.options = options.into_iter().map(Into::into).collect();
        self
    }
}

/// Tool call template associated with a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallTemplate {
    /// Target tool name.
    pub tool_name: String,
    /// `minijinja` template rendering tool arguments as JSON.
    pub arguments_template: String,
}

impl ToolCallTemplate {
    /// Creates a tool call template.
    #[must_use]
    pub fn new(tool_name: impl Into<String>, arguments_template: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            arguments_template: arguments_template.into(),
        }
    }
}

/// Slash-command definition used by registries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlashCommandDefinition {
    /// Command name without the leading slash.
    pub command: String,
    /// Human-readable description.
    pub description: String,
    /// Expansion template rendered for metadata and audit.
    pub expansion_template: String,
    /// Parameter definitions.
    #[serde(default)]
    pub parameters: Vec<CommandParameterSpec>,
    /// Deterministic tool call templates.
    #[serde(default)]
    pub tool_calls: Vec<ToolCallTemplate>,
}

impl SlashCommandDefinition {
    /// Creates a command definition.
    #[must_use]
    pub fn new(
        command: impl Into<String>,
        description: impl Into<String>,
        expansion_template: impl Into<String>,
    ) -> Self {
        Self {
            command: command.into().to_ascii_lowercase(),
            description: description.into(),
            expansion_template: expansion_template.into(),
            parameters: Vec::new(),
            tool_calls: Vec::new(),
        }
    }

    /// Adds a parameter specification.
    #[must_use]
    pub fn with_parameter(mut self, parameter: CommandParameterSpec) -> Self {
        self.parameters.push(parameter);
        self
    }

    /// Adds a tool call template.
    #[must_use]
    pub fn with_tool_call(mut self, tool_call: ToolCallTemplate) -> Self {
        self.tool_calls.push(tool_call);
        self
    }

    /// Validates and converts raw invocation parameters.
    ///
    /// # Errors
    ///
    /// Returns [`SlashCommandError`] when parameters are missing, unknown, or
    /// invalid for the declared schema.
    pub fn validate_parameters(
        &self,
        provided: &BTreeMap<String, String>,
    ) -> Result<BTreeMap<String, Value>, SlashCommandError> {
        validate_parameter_definitions(&self.command, &self.parameters)?;

        for key in provided.keys() {
            if !self
                .parameters
                .iter()
                .any(|parameter| parameter.name == *key)
            {
                return Err(SlashCommandError::UnknownParameter {
                    command: self.command.clone(),
                    parameter: key.clone(),
                });
            }
        }

        let mut typed = BTreeMap::new();
        for parameter in &self.parameters {
            match provided.get(&parameter.name) {
                Some(raw) => {
                    let value = parse_parameter_value(&self.command, parameter, raw)?;
                    typed.insert(parameter.name.clone(), value);
                }
                None if parameter.required => {
                    return Err(SlashCommandError::MissingRequiredParameter {
                        command: self.command.clone(),
                        parameter: parameter.name.clone(),
                    });
                }
                None => {
                    typed.insert(parameter.name.clone(), Value::Null);
                }
            }
        }

        Ok(typed)
    }
}

fn validate_parameter_definitions(
    command: &str,
    parameters: &[CommandParameterSpec],
) -> Result<(), SlashCommandError> {
    let mut names = HashSet::new();
    for parameter in parameters {
        if !names.insert(parameter.name.clone()) {
            return Err(SlashCommandError::InvalidParameterDefinition {
                command: command.to_owned(),
                parameter: parameter.name.clone(),
                reason: "duplicate parameter definition".to_owned(),
            });
        }
        if matches!(parameter.parameter_type, CommandParameterType::Select)
            && parameter.options.is_empty()
        {
            return Err(SlashCommandError::InvalidParameterDefinition {
                command: command.to_owned(),
                parameter: parameter.name.clone(),
                reason: "select parameters must provide options".to_owned(),
            });
        }
    }
    Ok(())
}

fn parse_parameter_value(
    command: &str,
    parameter: &CommandParameterSpec,
    raw: &str,
) -> Result<Value, SlashCommandError> {
    match parameter.parameter_type {
        CommandParameterType::String => Ok(Value::String(raw.to_owned())),
        CommandParameterType::Number => {
            if let Ok(signed) = raw.parse::<i64>() {
                return Ok(Value::Number(Number::from(signed)));
            }
            if let Ok(unsigned) = raw.parse::<u64>() {
                return Ok(Value::Number(Number::from(unsigned)));
            }
            Err(SlashCommandError::InvalidParameterValue {
                command: command.to_owned(),
                parameter: parameter.name.clone(),
                reason: "expected an integer".to_owned(),
            })
        }
        CommandParameterType::Boolean => match raw.to_ascii_lowercase().as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err(SlashCommandError::InvalidParameterValue {
                command: command.to_owned(),
                parameter: parameter.name.clone(),
                reason: "expected true or false".to_owned(),
            }),
        },
        CommandParameterType::Select => {
            if parameter.options.iter().any(|option| option == raw) {
                Ok(Value::String(raw.to_owned()))
            } else {
                Err(SlashCommandError::InvalidParameterValue {
                    command: command.to_owned(),
                    parameter: parameter.name.clone(),
                    reason: format!("expected one of [{}]", parameter.options.join(", ")),
                })
            }
        }
    }
}
