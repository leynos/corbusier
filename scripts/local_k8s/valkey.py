"""Valkey operator installation and instance helpers."""

from __future__ import annotations

import json

from plumbum import FG, local

from local_k8s.config import Config
from local_k8s.k8s import apply_manifest, ensure_namespace, read_secret_field, wait_for_pods_ready


def install_valkey_operator(cfg: Config, env: dict[str, str]) -> None:
    """Install the Valkey operator via Helm."""
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
    """Create a single-instance Valkey resource for Corbusier."""
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
    """Wait for the Valkey pod to become ready."""
    wait_for_pods_ready(f"app.kubernetes.io/instance={cfg.valkey_name}", cfg.namespace, env, timeout=300)


def read_valkey_uri(cfg: Config, env: dict[str, str]) -> str:
    """Build a Valkey URI from the operator-managed Secret."""
    password = read_secret_field(cfg.valkey_name, "password", cfg.namespace, env)
    host = f"{cfg.valkey_name}.{cfg.namespace}.svc.cluster.local"
    return f"valkey://:{password}@{host}:6379"
