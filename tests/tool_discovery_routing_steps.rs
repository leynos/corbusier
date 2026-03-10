//! Behaviour tests for tool discovery and call routing.

#[path = "tool_discovery_routing_steps/mod.rs"]
mod tool_discovery_routing_steps_defs;

use rstest_bdd_macros::scenario;
use tool_discovery_routing_steps_defs::world::{ToolDiscoveryWorld, world};

#[scenario(
    path = "tests/features/tool_discovery_routing.feature",
    name = "Discover tools from a running MCP server"
)]
#[tokio::test(flavor = "multi_thread")]
async fn discover_tools_from_running_server(world: ToolDiscoveryWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/tool_discovery_routing.feature",
    name = "Route a tool call to the correct server"
)]
#[tokio::test(flavor = "multi_thread")]
async fn route_tool_call_to_correct_server(world: ToolDiscoveryWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/tool_discovery_routing.feature",
    name = "Tool becomes unavailable when server stops"
)]
#[tokio::test(flavor = "multi_thread")]
async fn tool_unavailable_when_server_stops(world: ToolDiscoveryWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/tool_discovery_routing.feature",
    name = "Unknown tool call is rejected"
)]
#[tokio::test(flavor = "multi_thread")]
async fn unknown_tool_call_rejected(world: ToolDiscoveryWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/tool_discovery_routing.feature",
    name = "Tool call stderr is captured in the log store"
)]
#[tokio::test(flavor = "multi_thread")]
async fn tool_call_stderr_captured(world: ToolDiscoveryWorld) {
    let _ = world;
}
