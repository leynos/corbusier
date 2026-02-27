//! In-memory slash-command registry adapter.

use std::collections::HashMap;

use crate::message::domain::{
    CommandParameterSpec, CommandParameterType, SlashCommandDefinition, ToolCallTemplate,
};
use crate::message::ports::slash_command::{
    SlashCommandRegistry, SlashCommandRegistryError, SlashCommandRegistryResult,
};

/// In-memory registry for slash-command definitions.
#[derive(Debug, Clone)]
pub struct InMemorySlashCommandRegistry {
    commands: HashMap<String, SlashCommandDefinition>,
}

impl InMemorySlashCommandRegistry {
    /// Creates a registry with built-in command definitions.
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: default_commands()
                .into_iter()
                .map(|definition| {
                    debug_assert!(
                        definition.validate_schema().is_ok(),
                        "built-in slash command definitions must remain valid",
                    );
                    (definition.command.to_ascii_lowercase(), definition)
                })
                .collect(),
        }
    }

    /// Creates a registry from supplied command definitions.
    ///
    /// # Errors
    ///
    /// Returns [`SlashCommandRegistryError::InvalidDefinition`] when duplicate
    /// command names are provided.
    pub fn with_commands(
        definitions: impl IntoIterator<Item = SlashCommandDefinition>,
    ) -> SlashCommandRegistryResult<Self> {
        let mut commands = HashMap::new();
        for mut definition in definitions {
            definition.command = definition.command.to_ascii_lowercase();
            definition
                .validate_schema()
                .map_err(|error| SlashCommandRegistryError::InvalidDefinition(error.to_string()))?;

            if commands
                .insert(definition.command.clone(), definition)
                .is_some()
            {
                return Err(SlashCommandRegistryError::InvalidDefinition(
                    "duplicate command definition".to_owned(),
                ));
            }
        }
        Ok(Self { commands })
    }
}

impl Default for InMemorySlashCommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SlashCommandRegistry for InMemorySlashCommandRegistry {
    fn find_by_name(
        &self,
        command: &str,
    ) -> SlashCommandRegistryResult<Option<SlashCommandDefinition>> {
        Ok(self.commands.get(&command.to_ascii_lowercase()).cloned())
    }

    fn list(&self) -> SlashCommandRegistryResult<Vec<SlashCommandDefinition>> {
        let mut commands: Vec<_> = self.commands.values().cloned().collect();
        commands.sort_by(|left, right| left.command.cmp(&right.command));
        Ok(commands)
    }
}

fn default_commands() -> Vec<SlashCommandDefinition> {
    vec![
        SlashCommandDefinition::new(
            "task",
            "Create or manage task lifecycle actions",
            "Execute task action {{ action }}{% if issue %} for issue {{ issue }}{% endif %}.",
        )
        .with_parameter(
            CommandParameterSpec::new("action", CommandParameterType::Select, true)
                .with_options(["start", "create", "status"]),
        )
        .with_parameter(CommandParameterSpec::new(
            "issue",
            CommandParameterType::String,
            false,
        ))
        .with_tool_call(ToolCallTemplate::new(
            "task_service",
            concat!(
                "{\"action\":{{ action | json_string }},",
                "\"issue\":{% if issue is none %}null{% else %}{{ issue | json_string }}{% endif %}}",
            ),
        )),
        SlashCommandDefinition::new(
            "review",
            "Manage review workflows",
            "Execute review action {{ action }}{% if include_summary %} with summary{% endif %}.",
        )
        .with_parameter(
            CommandParameterSpec::new("action", CommandParameterType::Select, true)
                .with_options(["sync", "respond", "summary"]),
        )
        .with_parameter(CommandParameterSpec::new(
            "include_summary",
            CommandParameterType::Boolean,
            false,
        ))
        .with_tool_call(ToolCallTemplate::new(
            "review_service",
            concat!(
                "{\"action\":{{ action | json_string }},",
                "\"include_summary\":",
                "{% if include_summary is none %}null{% else %}{{ include_summary }}{% endif %}}",
            ),
        )),
    ]
}
