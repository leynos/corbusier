//! Step definitions for tool discovery and routing BDD scenarios.

mod given;
mod then;
mod when;
pub mod world;

use std::sync::Arc;

use corbusier::tool_registry::{
    adapters::{
        InMemoryMcpServerHost, ObjectStoreLogAdapter, StubGovernance,
        memory::{InMemoryMcpServerRegistry, InMemoryToolCatalog},
    },
    domain::{LogRetentionPolicy, McpTransport},
    services::{
        McpServerLifecycleService, RegisterMcpServerRequest, ServicePorts,
        ToolDiscoveryRoutingService,
    },
};
use eyre::{WrapErr, eyre};
use mockable::DefaultClock;

use world::ToolDiscoveryWorld;

pub type TestLifecycleService =
    McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

pub type TestDiscoveryService = ToolDiscoveryRoutingService<
    InMemoryToolCatalog,
    InMemoryMcpServerRegistry,
    InMemoryMcpServerHost,
    StubGovernance,
    ObjectStoreLogAdapter,
    DefaultClock,
>;

/// Constructs a lifecycle service from the shared component `Arc`s.
pub fn lifecycle_service(
    registry: &Arc<InMemoryMcpServerRegistry>,
    host: &Arc<InMemoryMcpServerHost>,
    clock: &Arc<DefaultClock>,
) -> TestLifecycleService {
    McpServerLifecycleService::new(registry.clone(), host.clone(), clock.clone())
}

/// Constructs a discovery service from the shared component `Arc`s.
pub fn discovery_service(
    catalog: &Arc<InMemoryToolCatalog>,
    registry: &Arc<InMemoryMcpServerRegistry>,
    host: &Arc<InMemoryMcpServerHost>,
    clock: &Arc<DefaultClock>,
) -> TestDiscoveryService {
    ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog: catalog.clone(),
            registry: registry.clone(),
            host: host.clone(),
            governance: Arc::new(StubGovernance::allowing()),
            log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
        },
        LogRetentionPolicy::default(),
        clock.clone(),
    )
}

/// Blocks the current thread on a future using [`tokio::task::block_in_place`].
///
/// This calls [`tokio::runtime::Handle::current`] and
/// [`block_in_place`](tokio::task::block_in_place), so it **must** be
/// executed on a multi-threaded Tokio runtime.  Calling it from a
/// single-threaded (`current_thread`) runtime will panic.
pub fn run_async<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}

pub fn request_from_world(
    world: &ToolDiscoveryWorld,
) -> Result<RegisterMcpServerRequest, eyre::Report> {
    let command = world
        .pending_command
        .as_deref()
        .ok_or_else(|| eyre!("pending command should exist"))?;
    Ok(RegisterMcpServerRequest::new(
        world.pending_name()?,
        McpTransport::stdio(command).wrap_err("valid stdio transport expected")?,
    ))
}
