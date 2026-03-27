# 2.3.2 hook policy enforcement and audit capture: interface specification

This companion document holds the detailed interface sketches that were removed
from the primary execution plan (ExecPlan) to keep the roadmap-level file under
the repository 400-line limit.

## Interfaces and dependencies

The implementation should end with these stable repository-relative interfaces
or their close equivalents.

In `src/hook_engine/domain/trigger.rs`, define an additive execution-scope
model owned by the hook engine.

```rust
pub struct HookExecutionScope {
    pub task_id: Option<TaskId>,
    pub conversation_id: Option<ConversationId>,
    pub metadata: serde_json::Value,
}
```

In `src/hook_engine/ports/policy_audit.rs`, define the hook-owned audit query
contract.

```rust
#[async_trait::async_trait]
pub trait HookPolicyAuditRepository: Send + Sync {
    async fn store(
        &self,
        ctx: &RequestContext,
        event: &PolicyAuditEvent,
    ) -> HookPolicyAuditResult<()>;

    async fn find_by_task(
        &self,
        ctx: &RequestContext,
        task_id: TaskId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>>;

    async fn find_by_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>>;

    async fn find_by_trigger_context(
        &self,
        ctx: &RequestContext,
        trigger_context_id: TriggerContextId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>>;
}
```

In `src/tool_registry/ports/`, define a tool-plane-owned governance contract
that hides hook-engine details from the service layer.

```rust
#[async_trait::async_trait]
pub trait ToolExecutionGovernance: Send + Sync {
    async fn enforce_before_call(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
        entry: &CatalogEntry,
    ) -> ToolGovernanceResult<ToolGovernanceDecision>;

    async fn observe_after_call(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
        entry: &CatalogEntry,
        result: &ToolCallResult,
    ) -> ToolGovernanceResult<()>;
}
```

In `src/tool_registry/domain/routing.rs`, keep `ToolCallRequest` additive and
introduce a workflow-correlation scope rather than mutating `RequestContext`.

```rust
pub struct ToolExecutionScope {
    pub task_id: Option<TaskId>,
    pub conversation_id: Option<ConversationId>,
    pub metadata: serde_json::Value,
}
```

## Dependency notes

The only infrastructure dependencies needed are the existing Diesel,
PostgreSQL, `mockable`, `rstest`, `rstest-bdd`, and `pg-embedded-setup-unpriv`
tooling already present in the repository.
