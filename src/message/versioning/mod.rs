//! Schema versioning and event migration support.
//!
//! This module provides infrastructure for versioned events and schema
//! migrations, allowing the system to evolve message formats while
//! maintaining backwards compatibility with stored data.

pub mod event;
pub mod upgrader;

pub use event::{EventMetadata, VersionedEvent};
pub use upgrader::{EventUpgrader, MessageCreatedUpgrader, UpgraderRegistry};
