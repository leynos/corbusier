//! Unit tests for [`ObjectStoreLogAdapter`].

use super::*;
use crate::test_support::test_request_ctx;
use crate::tool_registry::domain::{LogCaptureContext, LogRetentionPolicy, McpServerId};
use crate::tool_registry::ports::SweepContext;
use chrono::Duration;
use mockable::{Clock, DefaultClock};
use rstest::{fixture, rstest};

#[fixture]
fn adapter() -> ObjectStoreLogAdapter {
    ObjectStoreLogAdapter::in_memory()
}

/// Stores a startup log entry and returns its metadata.
#[expect(
    clippy::too_many_arguments,
    reason = "test helper bundles all dependencies for concise call sites"
)]
async fn store_startup_entry(
    adapter: &ObjectStoreLogAdapter,
    ctx: &RequestContext,
    server_id: McpServerId,
    clock: &dyn Clock,
    retention: &LogRetentionPolicy,
) -> LogEntryMetadata {
    let capture_ctx = LogCaptureContext {
        clock,
        retention,
        tenant_id: ctx.tenant_id(),
    };
    let content = Bytes::from("test stderr output");
    let metadata = LogEntryMetadata::for_startup(server_id, content.len() as u64, &capture_ctx);
    adapter
        .store_log(ctx, &metadata, content, retention)
        .await
        .expect("store_log should succeed");
    metadata
}

#[rstest]
#[tokio::test]
async fn sweep_deletes_expired_entries(adapter: ObjectStoreLogAdapter) {
    let ctx = test_request_ctx();
    let server_id = McpServerId::new();
    let clock = DefaultClock;

    // Use a very short retention so entries expire quickly.
    let retention = LogRetentionPolicy {
        max_bytes_per_log: 1024,
        max_logs_per_server: 100,
        retention_period: Duration::seconds(1),
    };

    let metadata = store_startup_entry(&adapter, &ctx, server_id, &clock, &retention).await;

    // Advance time past expiry.
    let now = metadata.expires_at() + Duration::seconds(1);
    let sweep = SweepContext {
        policy: &retention,
        now,
        entry_metadata: &[], // service passes empty slice
    };

    let swept = adapter
        .sweep_expired(&ctx, server_id, &sweep)
        .await
        .expect("sweep should succeed");
    assert_eq!(swept, 1, "one expired entry should be swept");

    // Verify the blob is actually gone.
    let result = adapter.retrieve_log(&ctx, metadata.object_path()).await;
    assert!(result.is_err(), "blob should be deleted after sweep");
}

#[rstest]
#[tokio::test]
async fn sweep_does_not_delete_unexpired_entries(adapter: ObjectStoreLogAdapter) {
    let ctx = test_request_ctx();
    let server_id = McpServerId::new();
    let clock = DefaultClock;

    let retention = LogRetentionPolicy::default();
    let metadata = store_startup_entry(&adapter, &ctx, server_id, &clock, &retention).await;

    // Sweep with current time — entry should not be expired.
    let now = metadata.captured_at();
    let sweep = SweepContext {
        policy: &retention,
        now,
        entry_metadata: &[],
    };

    let swept = adapter
        .sweep_expired(&ctx, server_id, &sweep)
        .await
        .expect("sweep should succeed");
    assert_eq!(swept, 0, "no entries should be swept");

    // Blob should still be retrievable.
    let blob = adapter
        .retrieve_log(&ctx, metadata.object_path())
        .await
        .expect("blob should still exist");
    assert_eq!(blob.as_ref(), b"test stderr output");
}

#[rstest]
#[tokio::test]
async fn sweep_enforces_count_limit(adapter: ObjectStoreLogAdapter) {
    let ctx = test_request_ctx();
    let server_id = McpServerId::new();
    let clock = DefaultClock;

    let retention = LogRetentionPolicy {
        max_bytes_per_log: 1024,
        max_logs_per_server: 2,
        retention_period: Duration::days(7),
    };

    // Store three entries — one should be swept as excess.
    store_startup_entry(&adapter, &ctx, server_id, &clock, &retention).await;
    store_startup_entry(&adapter, &ctx, server_id, &clock, &retention).await;
    store_startup_entry(&adapter, &ctx, server_id, &clock, &retention).await;

    let now = clock.utc();
    let sweep = SweepContext {
        policy: &retention,
        now,
        entry_metadata: &[],
    };

    let swept = adapter
        .sweep_expired(&ctx, server_id, &sweep)
        .await
        .expect("sweep should succeed");
    assert_eq!(swept, 1, "one excess entry should be swept");

    // Verify that exactly two blobs remain.
    let remaining = adapter
        .list_logs_for_server(&ctx, server_id)
        .await
        .expect("list should succeed");
    assert_eq!(remaining.len(), 2, "two logs should remain after sweep");
}

#[rstest]
#[tokio::test]
async fn delete_log_removes_metadata(adapter: ObjectStoreLogAdapter) {
    let ctx = test_request_ctx();
    let server_id = McpServerId::new();
    let clock = DefaultClock;

    let retention = LogRetentionPolicy {
        max_bytes_per_log: 1024,
        max_logs_per_server: 100,
        retention_period: Duration::seconds(1),
    };

    let metadata = store_startup_entry(&adapter, &ctx, server_id, &clock, &retention).await;

    // Delete via the trait method.
    adapter
        .delete_log(&ctx, metadata.object_path())
        .await
        .expect("delete should succeed");

    // After deletion, a sweep with expired time should find nothing.
    let now = metadata.expires_at() + Duration::seconds(1);
    let sweep = SweepContext {
        policy: &retention,
        now,
        entry_metadata: &[],
    };

    let swept = adapter
        .sweep_expired(&ctx, server_id, &sweep)
        .await
        .expect("sweep should succeed");
    assert_eq!(swept, 0, "deleted entry should not be swept again");
}

#[rstest]
#[tokio::test]
async fn sweep_only_affects_target_server(adapter: ObjectStoreLogAdapter) {
    let ctx = test_request_ctx();
    let server_a = McpServerId::new();
    let server_b = McpServerId::new();
    let clock = DefaultClock;

    let retention = LogRetentionPolicy {
        max_bytes_per_log: 1024,
        max_logs_per_server: 100,
        retention_period: Duration::seconds(1),
    };

    let meta_a = store_startup_entry(&adapter, &ctx, server_a, &clock, &retention).await;
    store_startup_entry(&adapter, &ctx, server_b, &clock, &retention).await;

    // Sweep only server A with expired time.
    let now = meta_a.expires_at() + Duration::seconds(1);
    let sweep = SweepContext {
        policy: &retention,
        now,
        entry_metadata: &[],
    };

    let swept = adapter
        .sweep_expired(&ctx, server_a, &sweep)
        .await
        .expect("sweep should succeed");
    assert_eq!(swept, 1, "only server A's entry should be swept");

    // Server B's log should still exist.
    let remaining = adapter
        .list_logs_for_server(&ctx, server_b)
        .await
        .expect("list should succeed");
    assert_eq!(remaining.len(), 1, "server B's log should remain");
}
