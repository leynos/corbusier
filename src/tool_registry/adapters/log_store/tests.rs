//! Unit tests for [`ObjectStoreLogAdapter`].

use super::*;
use crate::test_support::test_request_ctx;
use crate::tool_registry::domain::{LogCaptureContext, LogRetentionPolicy, McpServerId};
use crate::tool_registry::ports::{StoreLogRequest, SweepContext};
use chrono::Duration;
use mockable::{Clock, DefaultClock};
use object_store::memory::InMemory;
use rstest::{fixture, rstest};
use std::sync::Arc;

#[fixture]
fn adapter() -> ObjectStoreLogAdapter {
    ObjectStoreLogAdapter::in_memory()
}

/// Stores a startup log entry and returns its metadata.
async fn store_startup_entry(
    adapter: &ObjectStoreLogAdapter,
    ctx: &RequestContext,
    server_id: McpServerId,
    capture_ctx: &LogCaptureContext<'_>,
) -> ToolLogStoreResult<LogEntryMetadata> {
    let content = Bytes::from("test stderr output");
    let metadata = LogEntryMetadata::for_startup(server_id, content.len() as u64, capture_ctx);
    let request = StoreLogRequest {
        metadata: &metadata,
        content,
        retention: capture_ctx.retention,
    };
    adapter.store_log(ctx, &request).await?;
    Ok(metadata)
}

#[rstest]
#[tokio::test]
async fn sweep_deletes_expired_entries(adapter: ObjectStoreLogAdapter) -> ToolLogStoreResult<()> {
    let ctx = test_request_ctx();
    let server_id = McpServerId::new();
    let clock = DefaultClock;

    // Use a very short retention so entries expire quickly.
    let retention = LogRetentionPolicy {
        max_bytes_per_log: 1024,
        max_logs_per_server: 100,
        retention_period: Duration::seconds(1),
    };

    let capture_ctx = LogCaptureContext {
        clock: &clock,
        retention: &retention,
        tenant_id: ctx.tenant_id(),
    };
    let metadata = store_startup_entry(&adapter, &ctx, server_id, &capture_ctx).await?;

    // Advance time past expiry.
    let now = metadata.expires_at() + Duration::seconds(1);
    let sweep = SweepContext {
        policy: &retention,
        now,
    };

    let swept = adapter.sweep_expired(&ctx, server_id, &sweep).await?;
    assert_eq!(swept, 1, "one expired entry should be swept");

    // Verify the blob is actually gone.
    let result = adapter.retrieve_log(&ctx, metadata.object_path()).await;
    assert!(result.is_err(), "blob should be deleted after sweep");
    Ok(())
}

#[rstest]
#[tokio::test]
async fn sweep_does_not_delete_unexpired_entries(
    adapter: ObjectStoreLogAdapter,
) -> ToolLogStoreResult<()> {
    let ctx = test_request_ctx();
    let server_id = McpServerId::new();
    let clock = DefaultClock;

    let retention = LogRetentionPolicy::default();
    let capture_ctx = LogCaptureContext {
        clock: &clock,
        retention: &retention,
        tenant_id: ctx.tenant_id(),
    };
    let metadata = store_startup_entry(&adapter, &ctx, server_id, &capture_ctx).await?;

    // Sweep with current time — entry should not be expired.
    let now = metadata.captured_at();
    let sweep = SweepContext {
        policy: &retention,
        now,
    };

    let swept = adapter.sweep_expired(&ctx, server_id, &sweep).await?;
    assert_eq!(swept, 0, "no entries should be swept");

    // Blob should still be retrievable.
    let blob = adapter.retrieve_log(&ctx, metadata.object_path()).await?;
    assert_eq!(blob.as_ref(), b"test stderr output");
    Ok(())
}

#[rstest]
#[tokio::test]
async fn sweep_enforces_count_limit(adapter: ObjectStoreLogAdapter) -> ToolLogStoreResult<()> {
    let ctx = test_request_ctx();
    let server_id = McpServerId::new();
    let clock = DefaultClock;

    let retention = LogRetentionPolicy {
        max_bytes_per_log: 1024,
        max_logs_per_server: 2,
        retention_period: Duration::days(7),
    };

    let capture_ctx = LogCaptureContext {
        clock: &clock,
        retention: &retention,
        tenant_id: ctx.tenant_id(),
    };

    // Store three entries — one should be swept as excess.
    store_startup_entry(&adapter, &ctx, server_id, &capture_ctx).await?;
    store_startup_entry(&adapter, &ctx, server_id, &capture_ctx).await?;
    store_startup_entry(&adapter, &ctx, server_id, &capture_ctx).await?;

    let now = clock.utc();
    let sweep = SweepContext {
        policy: &retention,
        now,
    };

    let swept = adapter.sweep_expired(&ctx, server_id, &sweep).await?;
    assert_eq!(swept, 1, "one excess entry should be swept");

    // Verify that exactly two blobs remain.
    let remaining = adapter.list_logs_for_server(&ctx, server_id).await?;
    assert_eq!(remaining.len(), 2, "two logs should remain after sweep");
    Ok(())
}

#[rstest]
#[tokio::test]
async fn delete_log_removes_metadata(adapter: ObjectStoreLogAdapter) -> ToolLogStoreResult<()> {
    let ctx = test_request_ctx();
    let server_id = McpServerId::new();
    let clock = DefaultClock;

    let retention = LogRetentionPolicy {
        max_bytes_per_log: 1024,
        max_logs_per_server: 100,
        retention_period: Duration::seconds(1),
    };

    let capture_ctx = LogCaptureContext {
        clock: &clock,
        retention: &retention,
        tenant_id: ctx.tenant_id(),
    };
    let metadata = store_startup_entry(&adapter, &ctx, server_id, &capture_ctx).await?;

    // Delete via the trait method.
    adapter.delete_log(&ctx, metadata.object_path()).await?;

    // After deletion, a sweep with expired time should find nothing.
    let now = metadata.expires_at() + Duration::seconds(1);
    let sweep = SweepContext {
        policy: &retention,
        now,
    };

    let swept = adapter.sweep_expired(&ctx, server_id, &sweep).await?;
    assert_eq!(swept, 0, "deleted entry should not be swept again");
    Ok(())
}

#[rstest]
#[tokio::test]
async fn sweep_only_affects_target_server(
    adapter: ObjectStoreLogAdapter,
) -> ToolLogStoreResult<()> {
    let ctx = test_request_ctx();
    let server_a = McpServerId::new();
    let server_b = McpServerId::new();
    let clock = DefaultClock;

    let retention = LogRetentionPolicy {
        max_bytes_per_log: 1024,
        max_logs_per_server: 100,
        retention_period: Duration::seconds(1),
    };

    let capture_ctx = LogCaptureContext {
        clock: &clock,
        retention: &retention,
        tenant_id: ctx.tenant_id(),
    };
    let meta_a = store_startup_entry(&adapter, &ctx, server_a, &capture_ctx).await?;
    store_startup_entry(&adapter, &ctx, server_b, &capture_ctx).await?;

    // Sweep only server A with expired time.
    let now = meta_a.expires_at() + Duration::seconds(1);
    let sweep = SweepContext {
        policy: &retention,
        now,
    };

    let swept = adapter.sweep_expired(&ctx, server_a, &sweep).await?;
    assert_eq!(swept, 1, "only server A's entry should be swept");

    // Server B's log should still exist.
    let remaining = adapter.list_logs_for_server(&ctx, server_b).await?;
    assert_eq!(remaining.len(), 1, "server B's log should remain");
    Ok(())
}

#[tokio::test]
async fn sweep_rebuilds_metadata_from_object_store_after_restart() {
    let store = Arc::new(InMemory::new());
    let writer = ObjectStoreLogAdapter::new(store.clone());
    let reader = ObjectStoreLogAdapter::new(store);
    let ctx = test_request_ctx();
    let server_id = McpServerId::new();
    let clock = DefaultClock;

    let retention = LogRetentionPolicy {
        max_bytes_per_log: 1024,
        max_logs_per_server: 100,
        retention_period: Duration::seconds(1),
    };
    let capture_ctx = LogCaptureContext {
        clock: &clock,
        retention: &retention,
        tenant_id: ctx.tenant_id(),
    };
    let metadata = store_startup_entry(&writer, &ctx, server_id, &capture_ctx)
        .await
        .expect("startup log should be stored");

    let sweep = SweepContext {
        policy: &retention,
        now: metadata.expires_at() + Duration::seconds(1),
    };
    let swept = reader
        .sweep_expired(&ctx, server_id, &sweep)
        .await
        .expect("rebuilt metadata should allow sweep");
    assert_eq!(
        swept, 1,
        "rebuilt metadata should allow the expired log to be swept"
    );

    let result = writer.retrieve_log(&ctx, metadata.object_path()).await;
    assert!(
        result.is_err(),
        "rebuilt sweep should delete the persisted blob"
    );
}
