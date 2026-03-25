"""k3d cluster lifecycle helpers."""

from __future__ import annotations

import json

from plumbum import FG, local
from plumbum.commands.processes import ProcessExecutionError


def _run_json(*args: str) -> list[dict[str, object]] | dict[str, object] | None:
    """Run a k3d command that emits JSON and parse the result."""
    try:
        output = local["k3d"][*args, "-o", "json"]().strip()
    except ProcessExecutionError:
        return None
    if not output:
        return None
    try:
        parsed = json.loads(output)
    except json.JSONDecodeError:
        return None
    if isinstance(parsed, dict):
        return parsed
    if isinstance(parsed, list) and all(isinstance(item, dict) for item in parsed):
        return parsed
    return None


def cluster_exists(cluster_name: str) -> bool:
    """Check whether a cluster exists."""
    clusters = _run_json("cluster", "list")
    if not isinstance(clusters, list):
        return False
    return any(cluster.get("name") == cluster_name for cluster in clusters)


def get_cluster_ingress_port(cluster_name: str) -> int | None:
    """Read the host ingress port from a k3d cluster definition."""
    clusters = _run_json("cluster", "list")
    if not isinstance(clusters, list):
        return None

    for cluster in clusters:
        if cluster.get("name") != cluster_name:
            continue
        nodes = cluster.get("nodes")
        if not isinstance(nodes, list):
            return None
        for node in nodes:
            if not isinstance(node, dict):
                continue
            port_mappings = node.get("portMappings")
            if not isinstance(port_mappings, dict):
                continue
            for container_port, mappings in port_mappings.items():
                if container_port != "80/tcp" or not isinstance(mappings, list):
                    continue
                for mapping in mappings:
                    if not isinstance(mapping, dict):
                        continue
                    host_port = mapping.get("HostPort")
                    if isinstance(host_port, str) and host_port.isdigit():
                        return int(host_port)
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
