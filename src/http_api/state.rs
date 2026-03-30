//! Shared application state and adapter-local service traits.

use super::auth::BearerTokenAuthenticator;
use async_trait::async_trait;
use std::sync::Arc;

use crate::context::RequestContext;
use crate::message::{
    domain::{Conversation, ConversationId, Message},
    ports::{ConversationRepository, MessageRepository, MessageValidator},
    services::{
        AppendMessageRequest as AppendConversationMessageRequest, ConversationService,
        ConversationServiceError,
    },
};
use crate::task::{
    domain::{Task, TaskId},
    ports::TaskRepository,
    services::{
        AssociateBranchRequest, AssociatePullRequestRequest, CreateTaskFromIssueRequest,
        TaskLifecycleError, TaskLifecycleService, TransitionTaskRequest,
    },
};
use crate::tool_registry::{
    domain::{CatalogEntry, ToolCallRequest, ToolCallResult},
    ports::{
        McpServerHost, McpServerRegistryRepository, ToolCatalogRepository, ToolExecutionGovernance,
        ToolLogStore,
    },
    services::{ToolDiscoveryRoutingService, ToolDiscoveryRoutingServiceError},
};
use mockable::Clock;

/// Conversation operations exposed to the HTTP adapter.
#[async_trait]
pub trait ConversationApplication: Send + Sync {
    /// Creates a new conversation.
    async fn create_conversation(
        &self,
        ctx: &RequestContext,
    ) -> Result<Conversation, ConversationServiceError>;

    /// Returns ordered conversation history.
    async fn history(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> Result<Vec<Message>, ConversationServiceError>;

    /// Appends a message to an existing conversation.
    async fn append_message(
        &self,
        ctx: &RequestContext,
        request: AppendConversationMessageRequest,
    ) -> Result<Message, ConversationServiceError>;
}

/// Task operations exposed to the HTTP adapter.
#[async_trait]
pub trait TaskApplication: Send + Sync {
    /// Creates a task from issue metadata.
    async fn create_task(
        &self,
        ctx: &RequestContext,
        request: CreateTaskFromIssueRequest,
    ) -> Result<Task, TaskLifecycleError>;

    /// Retrieves a task by identifier.
    async fn get_task(
        &self,
        ctx: &RequestContext,
        task_id: TaskId,
    ) -> Result<Task, TaskLifecycleError>;

    /// Transitions a task state.
    async fn transition_task(
        &self,
        ctx: &RequestContext,
        request: TransitionTaskRequest,
    ) -> Result<Task, TaskLifecycleError>;

    /// Associates a branch with a task.
    async fn associate_branch(
        &self,
        ctx: &RequestContext,
        request: AssociateBranchRequest,
    ) -> Result<Task, TaskLifecycleError>;

    /// Associates a pull request with a task.
    async fn associate_pull_request(
        &self,
        ctx: &RequestContext,
        request: AssociatePullRequestRequest,
    ) -> Result<Task, TaskLifecycleError>;
}

/// Tool operations exposed to the HTTP adapter.
#[async_trait]
pub trait ToolApplication: Send + Sync {
    /// Lists the persisted tool catalog.
    async fn list_tools(
        &self,
        ctx: &RequestContext,
    ) -> Result<Vec<CatalogEntry>, ToolDiscoveryRoutingServiceError>;

    /// Routes a tool call.
    async fn call_tool(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
    ) -> Result<ToolCallResult, ToolDiscoveryRoutingServiceError>;
}

/// Shared state injected into the Actix application.
#[derive(Clone)]
pub struct ApiState {
    /// Conversation application service.
    pub conversations: Arc<dyn ConversationApplication>,
    /// Task application service.
    pub tasks: Arc<dyn TaskApplication>,
    /// Tool application service.
    pub tools: Arc<dyn ToolApplication>,
    /// Bearer-token authenticator.
    pub authenticator: BearerTokenAuthenticator,
}

impl ApiState {
    /// Creates a new shared API state bundle.
    #[must_use]
    pub fn new(
        conversations: Arc<dyn ConversationApplication>,
        tasks: Arc<dyn TaskApplication>,
        tools: Arc<dyn ToolApplication>,
        authenticator: BearerTokenAuthenticator,
    ) -> Self {
        Self {
            conversations,
            tasks,
            tools,
            authenticator,
        }
    }
}

#[async_trait]
impl<ConvoRepo, MessageRepo, Validator, C> ConversationApplication
    for ConversationService<ConvoRepo, MessageRepo, Validator, C>
where
    ConvoRepo: ConversationRepository + 'static,
    MessageRepo: MessageRepository + 'static,
    Validator: MessageValidator + 'static,
    C: Clock + Send + Sync + 'static,
{
    async fn create_conversation(
        &self,
        ctx: &RequestContext,
    ) -> Result<Conversation, ConversationServiceError> {
        self.create_conversation(ctx).await
    }

    async fn history(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> Result<Vec<Message>, ConversationServiceError> {
        self.history(ctx, conversation_id).await
    }

    async fn append_message(
        &self,
        ctx: &RequestContext,
        request: AppendConversationMessageRequest,
    ) -> Result<Message, ConversationServiceError> {
        self.append_message(ctx, request).await
    }
}

#[async_trait]
impl<R, C> TaskApplication for TaskLifecycleService<R, C>
where
    R: TaskRepository + 'static,
    C: Clock + Send + Sync + 'static,
{
    async fn create_task(
        &self,
        ctx: &RequestContext,
        request: CreateTaskFromIssueRequest,
    ) -> Result<Task, TaskLifecycleError> {
        self.create_from_issue(ctx, request).await
    }

    async fn get_task(
        &self,
        ctx: &RequestContext,
        task_id: TaskId,
    ) -> Result<Task, TaskLifecycleError> {
        self.get_by_id(ctx, task_id).await
    }

    async fn transition_task(
        &self,
        ctx: &RequestContext,
        request: TransitionTaskRequest,
    ) -> Result<Task, TaskLifecycleError> {
        self.transition_task(ctx, request).await
    }

    async fn associate_branch(
        &self,
        ctx: &RequestContext,
        request: AssociateBranchRequest,
    ) -> Result<Task, TaskLifecycleError> {
        self.associate_branch(ctx, request).await
    }

    async fn associate_pull_request(
        &self,
        ctx: &RequestContext,
        request: AssociatePullRequestRequest,
    ) -> Result<Task, TaskLifecycleError> {
        self.associate_pull_request(ctx, request).await
    }
}

#[async_trait]
impl<Cat, Reg, Host, Policy, Log, C> ToolApplication
    for ToolDiscoveryRoutingService<Cat, Reg, Host, Policy, Log, C>
where
    Cat: ToolCatalogRepository + 'static,
    Reg: McpServerRegistryRepository + 'static,
    Host: McpServerHost + 'static,
    Policy: ToolExecutionGovernance + 'static,
    Log: ToolLogStore + 'static,
    C: Clock + Send + Sync + 'static,
{
    async fn list_tools(
        &self,
        ctx: &RequestContext,
    ) -> Result<Vec<CatalogEntry>, ToolDiscoveryRoutingServiceError> {
        self.list_catalog(ctx).await
    }

    async fn call_tool(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
    ) -> Result<ToolCallResult, ToolDiscoveryRoutingServiceError> {
        self.call_tool(ctx, request).await
    }
}
