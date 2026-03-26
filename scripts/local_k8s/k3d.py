"""Helpers for managing local `k3d` clusters used by Corbusier previews.

This module wraps the `k3d` CLI operations needed by the local preview
workflow. It provides helpers for listing clusters, checking whether a cluster
exists, inspecting the loopback ingress port, writing `KUBECONFIG` overrides,
and importing locally built images into a cluster.

Unlike earlier probe-and-fallback helpers, these functions now treat command
failures and malformed JSON output as operational errors rather than as
"cluster not found" signals. Callers should surface those failures to the user
so local preview setup does not silently continue with stale assumptions.

Examples
--------
Check whether the default preview cluster exists:

>>> cluster_exists("corbusier-local")
False
"""

from __future__ import annotations

import json
from typing import Any

from plumbum import FG, local
from plumbum.commands.processes import ProcessExecutionError

from local_k8s.validation import LocalK8sError


def _k3d_cluster_list_output() -> str:
    """Run `k3d cluster list -o json` and return the stripped stdout."""
    try:
        return local["k3d"]["cluster", "list", "-o", "json"]().strip()
    except ProcessExecutionError as error:
        command_output = "\n".join(
            part
            for part in (str(error.stdout).strip(), str(error.stderr).strip())
            if part
        )
        raise LocalK8sError(
            "Failed to list k3d clusters"
            + (f": {command_output}" if command_output else "")
        ) from error


def _parse_k3d_cluster_list(output: str) -> list[dict[str, Any]]:
    """Parse and validate the JSON output of `k3d cluster list -o json`."""
    if not output:
        raise LocalK8sError(
            "Failed to list k3d clusters: command returned empty output"
        )
    try:
        parsed = json.loads(output)
    except json.JSONDecodeError as error:
        raise LocalK8sError(
            f"Failed to parse k3d cluster list JSON: {error}; output was: {output}"
        ) from error
    return _validate_k3d_cluster_list_shape(parsed)


def _validate_k3d_cluster_list_shape(parsed: Any) -> list[dict[str, Any]]:
    """Validate the parsed `k3d cluster list` JSON shape."""
    if not isinstance(parsed, list):
        raise LocalK8sError(
            f"Failed to parse k3d cluster list: expected a list, got "
            f"{type(parsed).__name__}"
        )
    if not all(isinstance(item, dict) for item in parsed):
        raise LocalK8sError(
            "Failed to parse k3d cluster list: expected every cluster record "
            "to be an object"
        )
    return parsed


def list_clusters() -> list[dict[str, Any]]:
    """Return parsed cluster records from `k3d cluster list`.

    Returns
    -------
    list[dict[str, Any]]
        Parsed cluster records emitted by `k3d cluster list -o json`. A
        genuine empty cluster list returns `[]`.

    Raises
    ------
    local_k8s.validation.LocalK8sError
        Raised when `k3d` fails, emits empty output, or returns JSON that does
        not match the expected list-of-dicts structure.
    """
    return _parse_k3d_cluster_list(_k3d_cluster_list_output())


def _ingress_port_from_mappings(mappings: list[Any]) -> int | None:
    """Return the first loopback HostPort from a list of port-mapping dicts."""
    for mapping in mappings:
        if not isinstance(mapping, dict):
            continue
        if mapping.get("HostIp") != "127.0.0.1":
            continue
        host_port = mapping.get("HostPort")
        if isinstance(host_port, str) and host_port.isdigit():
            return int(host_port)
    return None


def _ingress_port_from_node(node: dict[str, Any]) -> int | None:
    """Return the loopback ingress port for a single k3d node, or None."""
    port_mappings = node.get("portMappings")
    if not isinstance(port_mappings, dict):
        return None
    mappings = port_mappings.get("80/tcp")
    if not isinstance(mappings, list):
        return None
    return _ingress_port_from_mappings(mappings)


def _ingress_port_from_cluster(cluster: dict[str, Any]) -> int | None:
    """Extract the loopback ingress port from a parsed k3d cluster record."""
    nodes = cluster.get("nodes")
    if not isinstance(nodes, list):
        return None

    for node in nodes:
        if not isinstance(node, dict):
            continue
        port = _ingress_port_from_node(node)
        if port is not None:
            return port
    return None


def cluster_exists(cluster_name: str) -> bool:
    """Check whether a cluster exists.

    Parameters
    ----------
    cluster_name : str
        Name of the `k3d` cluster to look up.

    Returns
    -------
    bool
        `True` when a cluster with the requested name exists, otherwise
        `False`.

    Raises
    ------
    local_k8s.validation.LocalK8sError
        Raised when cluster listing fails or produces malformed output.
    """
    return any(cluster.get("name") == cluster_name for cluster in list_clusters())


def get_cluster_ingress_port(cluster_name: str) -> int | None:
    """Read the host ingress port from a `k3d` cluster definition.

    Parameters
    ----------
    cluster_name : str
        Name of the `k3d` cluster to inspect.

    Returns
    -------
    int | None
        Loopback ingress port when the cluster exists and exposes one,
        otherwise `None`.

    Raises
    ------
    local_k8s.validation.LocalK8sError
        Raised when cluster listing fails or produces malformed output.
    """
    for cluster in list_clusters():
        if cluster.get("name") != cluster_name:
            continue
        return _ingress_port_from_cluster(cluster)
    return None


def create_k3d_cluster(cluster_name: str, ingress_port: int) -> None:
    """Create a `k3d` cluster with loopback-only ingress exposure.

    Parameters
    ----------
    cluster_name : str
        Name of the cluster to create.
    ingress_port : int
        Loopback host port to bind to container port `80` on the load
        balancer.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when `k3d cluster create` fails.
    """
    port_mapping = f"127.0.0.1:{ingress_port}:80@loadbalancer"
    local["k3d"]["cluster", "create", cluster_name, "--agents", "1", "--port", port_mapping] & FG


def delete_k3d_cluster(cluster_name: str) -> None:
    """Delete a `k3d` cluster.

    Parameters
    ----------
    cluster_name : str
        Name of the cluster to delete.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when `k3d cluster delete` fails.
    """
    local["k3d"]["cluster", "delete", cluster_name] & FG


def kubeconfig_env(cluster_name: str) -> dict[str, str]:
    """Return an environment override pointing `kubectl` and `helm` at a cluster.

    Parameters
    ----------
    cluster_name : str
        Name of the cluster whose kubeconfig should be written.

    Returns
    -------
    dict[str, str]
        Environment override containing a `KUBECONFIG` path for the cluster.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when `k3d kubeconfig write` fails.
    """
    kubeconfig = local["k3d"]["kubeconfig", "write", cluster_name]().strip()
    return {"KUBECONFIG": kubeconfig}


def import_image_to_k3d(cluster_name: str, image_repo: str, image_tag: str) -> None:
    """Import a locally built image into a `k3d` cluster.

    Parameters
    ----------
    cluster_name : str
        Name of the cluster that should receive the image.
    image_repo : str
        Image repository name.
    image_tag : str
        Image tag to import.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when `k3d image import` fails.
    """
    local["k3d"]["image", "import", f"{image_repo}:{image_tag}", "-c", cluster_name] & FG
