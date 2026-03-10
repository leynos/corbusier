//! Internal types for lifecycle state transitions and compensation.

use crate::context::RequestContext;
use crate::tool_registry::{
    domain::McpServerRegistration,
    ports::{McpServerHost, McpServerHostError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LifecycleCompensationAction {
    Start,
    Stop,
}

#[derive(Debug, Clone)]
pub(super) struct LifecycleChange {
    pub(super) updated_server: McpServerRegistration,
    pub(super) compensation: Option<LifecycleCompensationAction>,
    pub(super) startup_stderr: Option<bytes::Bytes>,
}

impl LifecycleChange {
    pub(super) const fn without_compensation(updated_server: McpServerRegistration) -> Self {
        Self {
            updated_server,
            compensation: None,
            startup_stderr: None,
        }
    }

    pub(super) const fn with_compensation(
        updated_server: McpServerRegistration,
        compensation: LifecycleCompensationAction,
    ) -> Self {
        Self {
            updated_server,
            compensation: Some(compensation),
            startup_stderr: None,
        }
    }

    pub(super) fn with_startup_stderr(mut self, stderr: Option<bytes::Bytes>) -> Self {
        self.startup_stderr = stderr;
        self
    }
}

/// Identifies which host operation to invoke for a lifecycle transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LifecycleHostAction {
    Start,
    Stop,
}

impl LifecycleHostAction {
    /// Returns the compensation action for this host action.
    pub(super) const fn compensation(self) -> LifecycleCompensationAction {
        match self {
            Self::Start => LifecycleCompensationAction::Stop,
            Self::Stop => LifecycleCompensationAction::Start,
        }
    }

    /// Executes the host action, returning any startup stderr captured.
    pub(super) async fn execute<H: McpServerHost>(
        self,
        ctx: &RequestContext,
        host: &H,
        server: &McpServerRegistration,
    ) -> Result<Option<bytes::Bytes>, McpServerHostError> {
        match self {
            Self::Start => Ok(host.start(ctx, server).await?.stderr_output),
            Self::Stop => {
                host.stop(ctx, server).await?;
                Ok(None)
            }
        }
    }
}

pub(super) struct LifecycleTransition<DomainMut> {
    pub(super) host_action: LifecycleHostAction,
    pub(super) domain_mutation: DomainMut,
}

impl<DomainMut> LifecycleTransition<DomainMut> {
    pub(super) const fn new(host_action: LifecycleHostAction, domain_mutation: DomainMut) -> Self {
        Self {
            host_action,
            domain_mutation,
        }
    }
}
