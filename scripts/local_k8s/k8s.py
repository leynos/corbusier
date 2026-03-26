"""Kubernetes helper functions for local preview management.

This module wraps the `kubectl` calls used by the Corbusier local preview
workflow. The helpers assume the caller has already selected the correct
cluster context, typically by passing the `KUBECONFIG` override returned by the
local `k3d` utilities.

Typical usage keeps the functions small and composable: ensure the namespace
exists, apply manifests, wait for pods to become Ready, then read any
operator-managed Secret material needed to assemble application configuration.

Examples
--------
Ensure the application namespace exists before installing platform services:

>>> ensure_namespace("corbusier", {"KUBECONFIG": "/tmp/kubeconfig"})

Apply a generated manifest and wait for the resulting pods:

>>> apply_manifest('{"kind":"ConfigMap","apiVersion":"v1"}', {"KUBECONFIG": "/tmp/kubeconfig"})
>>> wait_for_pods_ready("app=example", "corbusier", {"KUBECONFIG": "/tmp/kubeconfig"})
"""

from __future__ import annotations

import json

from plumbum import FG, local

from local_k8s.validation import LocalK8sSecretError, b64decode_k8s_secret_field


def namespace_exists(namespace: str, env: dict[str, str]) -> bool:
    """Return whether a Kubernetes namespace already exists.

    Parameters
    ----------
    namespace : str
        Namespace name to query.
    env : dict[str, str]
        Environment overrides, typically containing `KUBECONFIG`.

    Returns
    -------
    bool
        `True` when the namespace exists, otherwise `False`.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when `kubectl` fails for a reason other than a missing
        namespace.
    """
    with local.env(**env):
        command = local["kubectl"]["get", "namespace", namespace]
        return command.run(retcode=None)[0] == 0


def create_namespace(namespace: str, env: dict[str, str]) -> None:
    """Create a namespace idempotently.

    Parameters
    ----------
    namespace : str
        Namespace to create.
    env : dict[str, str]
        Environment overrides, typically containing `KUBECONFIG`.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when `kubectl create namespace` or `kubectl apply` fails.
    """
    with local.env(**env):
        kubectl = local["kubectl"]
        manifest = kubectl["create", "namespace", namespace, "--dry-run=client", "-o", "yaml"]()
        kubectl["apply", "-f", "-"].run(stdin=manifest)


def ensure_namespace(namespace: str, env: dict[str, str]) -> None:
    """Ensure that a namespace exists before deploying resources.

    Parameters
    ----------
    namespace : str
        Namespace that should exist.
    env : dict[str, str]
        Environment overrides, typically containing `KUBECONFIG`.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when namespace creation fails.
    """
    if not namespace_exists(namespace, env):
        create_namespace(namespace, env)


def apply_manifest(manifest: str, env: dict[str, str]) -> None:
    """Apply a YAML or JSON manifest to the selected cluster.

    Parameters
    ----------
    manifest : str
        YAML or JSON manifest content passed to `kubectl apply -f -`.
    env : dict[str, str]
        Environment overrides, typically containing `KUBECONFIG`.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when `kubectl apply` fails.
    """
    with local.env(**env):
        local["kubectl"]["apply", "-f", "-"].run(stdin=manifest)


def wait_for_pods_ready(selector: str, namespace: str, env: dict[str, str], timeout: int = 300) -> None:
    """Wait for pods matching a selector to report the Ready condition.

    Parameters
    ----------
    selector : str
        Kubernetes label selector used to identify pods.
    namespace : str
        Namespace containing the target pods.
    env : dict[str, str]
        Environment overrides, typically containing `KUBECONFIG`.
    timeout : int, default=300
        Timeout in seconds passed to `kubectl wait`.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when the pods fail to become ready before the timeout or when
        `kubectl` returns an error.
    """
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
    """Read and decode a field from a Kubernetes Secret.

    Parameters
    ----------
    secret_name : str
        Secret resource name.
    field : str
        Field name expected under the Secret's `data` map.
    namespace : str
        Namespace containing the Secret.
    env : dict[str, str]
        Environment overrides, typically containing `KUBECONFIG`.

    Returns
    -------
    str
        UTF-8 decoded secret value.

    Raises
    ------
    LocalK8sSecretError
        Raised when the Secret is missing the requested field or the field is
        empty.
    SecretDecodeError
        Raised when the field content is not valid base64 or does not decode to
        UTF-8 text.
    plumbum.commands.processes.ProcessExecutionError
        Raised when `kubectl get secret` fails.
    """
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
