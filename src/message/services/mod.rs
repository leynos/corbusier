//! Application services for the message subsystem.
//!
//! Services orchestrate domain operations and coordinate between ports,
//! implementing business workflows that span multiple aggregates.

mod handoff;

pub use handoff::HandoffService;
