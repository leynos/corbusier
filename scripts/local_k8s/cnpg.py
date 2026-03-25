"""CloudNativePG installation and cluster helpers."""

from __future__ import annotations

import json

from plumbum import FG, local

from local_k8s.config import Config
from local_k8s.k8s import apply_manifest, ensure_namespace, read_secret_field, wait_for_pods_ready


def install_cnpg_operator(cfg: Config, env: dict[str, str]) -> None:
    """Install the CloudNativePG operator via Helm."""
    ensure_namespace(cfg.cnpg_namespace, env)
    with local.env(**env):
        helm = local["helm"]
        helm["repo", "add", "--force-update", "cnpg", "https://cloudnative-pg.github.io/charts"] & FG
        helm["repo", "update"] & FG
        helm[
            "upgrade",
            "--install",
            cfg.cnpg_release,
            "cnpg/cloudnative-pg",
            "--namespace",
            cfg.cnpg_namespace,
            "--wait",
        ] & FG


def create_cnpg_cluster(cfg: Config, env: dict[str, str]) -> None:
    """Create a single-instance Postgres cluster for Corbusier."""
    manifest = json.dumps(
        {
            "apiVersion": "postgresql.cnpg.io/v1",
            "kind": "Cluster",
            "metadata": {
                "name": cfg.pg_cluster_name,
                "namespace": cfg.namespace,
                "labels": {
                    "app.kubernetes.io/name": "cnpg-cluster",
                    "app.kubernetes.io/instance": cfg.pg_cluster_name,
                    "app.kubernetes.io/component": "database",
                },
            },
            "spec": {
                "instances": 1,
                "storage": {"size": "1Gi"},
                "bootstrap": {
                    "initdb": {
                        "database": "corbusier",
                        "owner": "corbusier",
                    }
                },
            },
        }
    )
    apply_manifest(manifest, env)


def wait_for_cnpg_ready(cfg: Config, env: dict[str, str]) -> None:
    """Wait for the Postgres cluster pods to become ready."""
    wait_for_pods_ready(f"cnpg.io/cluster={cfg.pg_cluster_name}", cfg.namespace, env, timeout=600)


def read_pg_app_uri(cfg: Config, env: dict[str, str]) -> str:
    """Read the application URI from CNPG's generated Secret."""
    return read_secret_field(f"{cfg.pg_cluster_name}-app", "uri", cfg.namespace, env)
