"""Valkey operator installation and instance helpers for local previews.

This module wraps the Kubernetes and Helm steps required to install the Valkey
operator, provision a single-instance Valkey resource for Corbusier, wait for
the resulting pods to become ready, and derive the runtime Redis-compatible URI
from the operator-managed `Secret`.

The functions are designed to be orchestrated by `local_k8s.orchestration`, but
they can also be reused independently when debugging a local preview
environment or replaying a single cache-related step.

Examples
--------
Install the Valkey operator into its management namespace:

>>> # install_valkey_operator(cfg, env)

Read the Redis-compatible URI after the instance is ready:

>>> # read_valkey_uri(cfg, env)
"""

from __future__ import annotations

import json

from plumbum import FG, local

from local_k8s.config import Config
from local_k8s.k8s import apply_manifest, ensure_namespace, read_secret_field, wait_for_pods_ready


def install_valkey_operator(cfg: Config, env: dict[str, str]) -> None:
    """Install the Valkey operator via Helm.

    Parameters
    ----------
    cfg : Config
        Local preview configuration describing the Valkey release and operator
        namespace.
    env : dict[str, str]
        Environment variables used to target the correct Kubernetes cluster.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when Helm repository or installation commands fail.
    """
    ensure_namespace(cfg.valkey_namespace, env)
    with local.env(**env):
        helm = local["helm"]
        helm[
            "repo",
            "add",
            "--force-update",
            "valkey-operator",
            "https://hyperspike.github.io/valkey-operator",
        ] & FG
        helm["repo", "update"] & FG
        helm[
            "upgrade",
            "--install",
            cfg.valkey_release,
            "valkey-operator/valkey-operator",
            "--namespace",
            cfg.valkey_namespace,
            "--wait",
        ] & FG


def create_valkey_instance(cfg: Config, env: dict[str, str]) -> None:
    """Create a single-instance Valkey resource for Corbusier.

    Parameters
    ----------
    cfg : Config
        Local preview configuration describing the Valkey instance name and
        namespace.
    env : dict[str, str]
        Environment variables used to target the correct Kubernetes cluster.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when applying the Valkey manifest fails.
    """
    manifest = json.dumps(
        {
            "apiVersion": "valkey.io/v1alpha1",
            "kind": "Valkey",
            "metadata": {
                "name": cfg.valkey_name,
                "namespace": cfg.namespace,
                "labels": {
                    "app.kubernetes.io/name": "valkey",
                    "app.kubernetes.io/instance": cfg.valkey_name,
                    "app.kubernetes.io/component": "cache",
                },
            },
            "spec": {
                "replicas": 1,
                "resources": {
                    "requests": {
                        "cpu": "50m",
                        "memory": "64Mi",
                    }
                },
            },
        }
    )
    apply_manifest(manifest, env)


def wait_for_valkey_ready(cfg: Config, env: dict[str, str]) -> None:
    """Wait for the Valkey pod to become ready.

    Parameters
    ----------
    cfg : Config
        Local preview configuration describing the Valkey instance label and
        namespace.
    env : dict[str, str]
        Environment variables used to target the correct Kubernetes cluster.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    TimeoutError
        Raised when the Valkey pod does not become ready before the timeout.
    plumbum.commands.processes.ProcessExecutionError
        Raised when the underlying `kubectl` wait command fails.
    """
    wait_for_pods_ready(f"app.kubernetes.io/instance={cfg.valkey_name}", cfg.namespace, env, timeout=300)


def read_valkey_uri(cfg: Config, env: dict[str, str]) -> str:
    """Build a Valkey URI from the operator-managed Secret.

    Parameters
    ----------
    cfg : Config
        Local preview configuration describing the Valkey instance name and
        namespace.
    env : dict[str, str]
        Environment variables used to target the correct Kubernetes cluster.

    Returns
    -------
    str
        Redis-compatible URI constructed from the operator-managed password and
        service hostname.

    Raises
    ------
    local_k8s.validation.LocalK8sSecretError
        Raised when the Valkey `Secret` is missing the required password field.
    local_k8s.validation.SecretDecodeError
        Raised when the password field cannot be decoded from base64.
    plumbum.commands.processes.ProcessExecutionError
        Raised when the Kubernetes `Secret` cannot be read.
    """
    password = read_secret_field(cfg.valkey_name, "password", cfg.namespace, env)
    host = f"{cfg.valkey_name}.{cfg.namespace}.svc.cluster.local"
    return f"redis://:{password}@{host}:6379"
