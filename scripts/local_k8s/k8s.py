"""Kubernetes helper functions for local preview management."""

from __future__ import annotations

import json

from plumbum import FG, local

from local_k8s.validation import LocalK8sSecretError, b64decode_k8s_secret_field


def namespace_exists(namespace: str, env: dict[str, str]) -> bool:
    """Check whether a namespace exists."""
    with local.env(**env):
        command = local["kubectl"]["get", "namespace", namespace]
        return command.run(retcode=None)[0] == 0


def create_namespace(namespace: str, env: dict[str, str]) -> None:
    """Create a namespace idempotently."""
    with local.env(**env):
        kubectl = local["kubectl"]
        manifest = kubectl["create", "namespace", namespace, "--dry-run=client", "-o", "yaml"]()
        kubectl["apply", "-f", "-"].run(stdin=manifest)


def ensure_namespace(namespace: str, env: dict[str, str]) -> None:
    """Ensure a namespace exists."""
    if not namespace_exists(namespace, env):
        create_namespace(namespace, env)


def apply_manifest(manifest: str, env: dict[str, str]) -> None:
    """Apply a YAML or JSON manifest via stdin."""
    with local.env(**env):
        local["kubectl"]["apply", "-f", "-"].run(stdin=manifest)


def wait_for_pods_ready(selector: str, namespace: str, env: dict[str, str], timeout: int = 300) -> None:
    """Wait for matching pods to report the Ready condition."""
    with local.env(**env):
        local["kubectl"][
            "wait",
            "--for=condition=Ready",
            "pod",
            f"--selector={selector}",
            f"--namespace={namespace}",
            f"--timeout={timeout}s",
        ] & FG


def read_secret_field(secret_name: str, field: str, namespace: str, env: dict[str, str]) -> str:
    """Read and decode a field from a Kubernetes Secret."""
    with local.env(**env):
        output = local["kubectl"][
            "get",
            "secret",
            secret_name,
            f"--namespace={namespace}",
            "-o",
            "json",
        ]()
    payload = json.loads(output)
    data = payload.get("data", {})
    if not isinstance(data, dict) or field not in data:
        raise LocalK8sSecretError(
            f"Secret '{secret_name}' in namespace '{namespace}' does not contain field '{field}'"
        )
    value = data[field]
    if not isinstance(value, str) or not value:
        raise LocalK8sSecretError(
            f"Secret '{secret_name}' field '{field}' in namespace '{namespace}' is empty"
        )
    return b64decode_k8s_secret_field(value)
