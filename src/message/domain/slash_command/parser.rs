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
    let mut tokenizer = Tokenizer::new();
    for character in input.chars() {
        tokenizer.push(character)?;
    }
    tokenizer.finish()
}

fn is_valid_identifier(value: &str) -> bool {
    value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

#[derive(Clone, Copy)]
enum ParseState {
    Unquoted,
    Quoted(char),
}

struct Tokenizer {
    tokens: Vec<String>,
    current: String,
    state: ParseState,
    escaped: bool,
}

impl Tokenizer {
    const fn new() -> Self {
        Self {
            tokens: Vec::new(),
            current: String::new(),
            state: ParseState::Unquoted,
            escaped: false,
        }
    }

    fn push(&mut self, character: char) -> Result<(), SlashCommandError> {
        if self.escaped {
            self.current.push(character);
            self.escaped = false;
            return Ok(());
        }

        match self.state {
            ParseState::Unquoted => self.push_unquoted(character),
            ParseState::Quoted(quote_char) => {
                self.push_quoted(character, quote_char);
                Ok(())
            }
        }
    }

    fn push_unquoted(&mut self, character: char) -> Result<(), SlashCommandError> {
        if character == '"' || character == '\'' {
            self.state = ParseState::Quoted(character);
            return Ok(());
        }
        if character.is_whitespace() {
            self.flush_current();
            return Ok(());
        }
        if character == '\\' {
            return Err(SlashCommandError::InvalidParameterToken {
                token: self.current.clone(),
            });
        }
        self.current.push(character);
        Ok(())
    }

    fn push_quoted(&mut self, character: char, quote_char: char) {
        if character == '\\' {
            self.escaped = true;
            return;
        }
        if character == quote_char {
            self.state = ParseState::Unquoted;
            return;
        }
        self.current.push(character);
    }

    fn flush_current(&mut self) {
        if !self.current.is_empty() {
            self.tokens.push(std::mem::take(&mut self.current));
        }
    }

    fn finish(mut self) -> Result<Vec<String>, SlashCommandError> {
        if matches!(self.state, ParseState::Quoted(_)) {
            return Err(SlashCommandError::UnterminatedQuotedValue);
        }
        if self.escaped {
            return Err(SlashCommandError::UnterminatedQuotedValue);
        }
        self.flush_current();
        Ok(self.tokens)
    }
}
