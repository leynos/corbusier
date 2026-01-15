//! Domain types for the message subsystem.
//!
//! This module contains pure domain types with no infrastructure dependencies.
//! All types are immutable after construction and serialisable via serde.

mod content;
mod ids;
mod message;
mod metadata;
mod role;

pub use content::{AttachmentPart, ContentPart, TextPart, ToolCallPart, ToolResultPart};
pub use ids::{ConversationId, MessageId, SequenceNumber, TurnId};
pub use message::{Message, MessageBuilder, MessageBuilderError};
pub use metadata::{MessageMetadata, SlashCommandExpansion};
pub use role::Role;
