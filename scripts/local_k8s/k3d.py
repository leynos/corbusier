"""k3d cluster lifecycle helpers."""

from __future__ import annotations

import json
from typing import Any

from plumbum import FG, local
from plumbum.commands.processes import ProcessExecutionError


def list_clusters() -> list[dict[str, Any]]:
    """Return `k3d cluster list` output, or an empty list on error."""
    try:
        output = local["k3d"]["cluster", "list", "-o", "json"]().strip()
    except ProcessExecutionError:
        return []
    if not output:
        return []
    try:
        parsed = json.loads(output)
    except json.JSONDecodeError:
        return []
    if isinstance(parsed, list) and all(isinstance(item, dict) for item in parsed):
        return parsed
    return []


def _ingress_port_from_cluster(cluster: dict[str, Any]) -> int | None:
    """Extract the loopback ingress port from a parsed k3d cluster record."""
    nodes = cluster.get("nodes")
    if not isinstance(nodes, list):
        return None

    for node in nodes:
        if not isinstance(node, dict):
            continue
        port_mappings = node.get("portMappings")
        if not isinstance(port_mappings, dict):
            continue
        mappings = port_mappings.get("80/tcp")
        if not isinstance(mappings, list):
            continue
        for mapping in mappings:
            if not isinstance(mapping, dict):
                continue
            host_port = mapping.get("HostPort")
            if isinstance(host_port, str) and host_port.isdigit():
                return int(host_port)
    return None


def cluster_exists(cluster_name: str) -> bool:
    """Check whether a cluster exists."""
    return any(cluster.get("name") == cluster_name for cluster in list_clusters())


def get_cluster_ingress_port(cluster_name: str) -> int | None:
    """Read the host ingress port from a k3d cluster definition."""
    for cluster in list_clusters():
        if cluster.get("name") != cluster_name:
            continue
        return _ingress_port_from_cluster(cluster)
    return None


def create_k3d_cluster(cluster_name: str, ingress_port: int) -> None:
    """Create a k3d cluster with loopback-only ingress exposure."""
    port_mapping = f"127.0.0.1:{ingress_port}:80@loadbalancer"
    local["k3d"]["cluster", "create", cluster_name, "--agents", "1", "--port", port_mapping] & FG


def delete_k3d_cluster(cluster_name: str) -> None:
    """Delete a k3d cluster."""
    local["k3d"]["cluster", "delete", cluster_name] & FG


def kubeconfig_env(cluster_name: str) -> dict[str, str]:
    """Return an environment override pointing kubectl/helm at the cluster."""
    kubeconfig = local["k3d"]["kubeconfig", "write", cluster_name]().strip()
    return {"KUBECONFIG": kubeconfig}


def import_image_to_k3d(cluster_name: str, image_repo: str, image_tag: str) -> None:
    """Import a locally built image into k3d."""
    local["k3d"]["image", "import", f"{image_repo}:{image_tag}", "-c", cluster_name] & FG
