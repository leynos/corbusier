//! Turn-session lifecycle status values.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Lifecycle status of a turn orchestration session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnSessionStatus {
    /// Session is active and can process additional turns.
    Active,
    /// Session slot is reserved while the runtime session is being created.
    Reserved,
    /// Session reached its expiry window and is no longer reusable.
    Expired,
}

impl TurnSessionStatus {
    /// Returns the canonical storage representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Reserved => "reserved",
            Self::Expired => "expired",
        }
    }
}

/// Error returned when parsing an invalid turn-session status value.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("unknown turn session status: {0}")]
pub struct ParseTurnSessionStatusError(pub String);

impl TryFrom<&str> for TurnSessionStatus {
    type Error = ParseTurnSessionStatusError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        parse_turn_session_status(value)
            .ok_or_else(|| ParseTurnSessionStatusError(value.to_owned()))
    }
}

fn parse_turn_session_status(value: &str) -> Option<TurnSessionStatus> {
    let normalized = value.trim();
    if normalized.eq_ignore_ascii_case("active") {
        Some(TurnSessionStatus::Active)
    } else if normalized.eq_ignore_ascii_case("reserved") {
        Some(TurnSessionStatus::Reserved)
    } else if normalized.eq_ignore_ascii_case("expired") {
        Some(TurnSessionStatus::Expired)
    } else {
        None
    }
}
