//! Slash-command parser.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::SlashCommandError;

/// A parsed slash-command invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlashCommandInvocation {
    command: String,
    parameters: BTreeMap<String, String>,
}

impl SlashCommandInvocation {
    /// Parses `/<command> key=value key2="quoted value"` input.
    ///
    /// # Errors
    ///
    /// Returns [`SlashCommandError`] when the input is empty or malformed.
    pub fn parse(raw_input: &str) -> Result<Self, SlashCommandError> {
        let trimmed = raw_input.trim();
        if trimmed.is_empty() {
            return Err(SlashCommandError::EmptyInput);
        }

        let tokens = tokenize(trimmed)?;
        let command_token = tokens.first().ok_or(SlashCommandError::EmptyInput)?;
        let command = parse_command_token(command_token)?;

        let mut parameters = BTreeMap::new();
        for token in tokens.iter().skip(1) {
            let (key, value) =
                token
                    .split_once('=')
                    .ok_or_else(|| SlashCommandError::InvalidParameterToken {
                        token: token.to_owned(),
                    })?;

            if key.is_empty() || !is_valid_identifier(key) {
                return Err(SlashCommandError::InvalidParameterToken {
                    token: token.to_owned(),
                });
            }

            let normalized_key = key.to_ascii_lowercase();
            if parameters
                .insert(normalized_key.clone(), value.to_owned())
                .is_some()
            {
                return Err(SlashCommandError::DuplicateParameter(normalized_key));
            }
        }

        Ok(Self {
            command,
            parameters,
        })
    }

    /// Returns the command name without the leading slash.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Returns parsed parameter values as raw strings.
    #[must_use]
    pub const fn parameters(&self) -> &BTreeMap<String, String> {
        &self.parameters
    }
}

fn parse_command_token(token: &str) -> Result<String, SlashCommandError> {
    let command = token
        .strip_prefix('/')
        .ok_or(SlashCommandError::MissingLeadingSlash)?;
    if command.is_empty() || !is_valid_identifier(command) {
        return Err(SlashCommandError::InvalidCommandName(command.to_owned()));
    }
    Ok(command.to_ascii_lowercase())
}

fn tokenize(input: &str) -> Result<Vec<String>, SlashCommandError> {
    let mut state = TokenizeState::new();

    for character in input.chars() {
        if let Some(quote_char) = state.in_quotes {
            process_quoted_character(character, quote_char, &mut state);
        } else {
            process_unquoted_character(character, &mut state)?;
        }
    }

    validate_final_state(state)
}

fn process_quoted_character(character: char, quote_char: char, state: &mut TokenizeState) {
    if state.escaped {
        state.current.push(character);
        state.escaped = false;
        return;
    }

    match character {
        '\\' => state.escaped = true,
        _ if character == quote_char => state.in_quotes = None,
        _ => state.current.push(character),
    }
}

fn process_unquoted_character(
    character: char,
    state: &mut TokenizeState,
) -> Result<(), SlashCommandError> {
    match character {
        '"' | '\'' => state.in_quotes = Some(character),
        _ if character.is_whitespace() => state.flush_current(),
        '\\' => {
            state.current.push(character);
            return Err(SlashCommandError::InvalidParameterToken {
                token: state.current.clone(),
            });
        }
        _ => state.current.push(character),
    }
    Ok(())
}

fn validate_final_state(mut state: TokenizeState) -> Result<Vec<String>, SlashCommandError> {
    if state.in_quotes.is_some() || state.escaped {
        return Err(SlashCommandError::UnterminatedQuotedValue);
    }
    state.flush_current();
    Ok(state.tokens)
}

struct TokenizeState {
    tokens: Vec<String>,
    current: String,
    in_quotes: Option<char>,
    escaped: bool,
}

impl TokenizeState {
    const fn new() -> Self {
        Self {
            tokens: Vec::new(),
            current: String::new(),
            in_quotes: None,
            escaped: false,
        }
    }

    fn flush_current(&mut self) {
        if !self.current.is_empty() {
            self.tokens.push(std::mem::take(&mut self.current));
        }
    }
}

fn is_valid_identifier(value: &str) -> bool {
    value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}
