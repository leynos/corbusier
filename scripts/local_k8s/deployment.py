"""Application deployment helpers for the local preview environment."""

from __future__ import annotations

import json

from plumbum import FG, local

from local_k8s.config import Config


def build_docker_image(cfg: Config) -> None:
    """Build the Corbusier image from the repository root."""
    with local.cwd(cfg.project_root):
        local["docker"]["build", "-t", f"{cfg.image_repo}:{cfg.image_tag}", "."] & FG


def create_app_secret(cfg: Config, env: dict[str, str], database_url: str, valkey_url: str) -> None:
    """Create or update the application Secret used by the Helm chart."""
    manifest = json.dumps(
        {
            "apiVersion": "v1",
            "kind": "Secret",
            "metadata": {
                "name": cfg.app_secret_name,
                "namespace": cfg.namespace,
                "labels": {
                    "app.kubernetes.io/name": cfg.app_name,
                    "app.kubernetes.io/instance": cfg.app_name,
                    "app.kubernetes.io/managed-by": "local_k8s",
                },
            },
            "stringData": {
                "DATABASE_URL": database_url,
                "VALKEY_URL": valkey_url,
            },
        }
    )
    with local.env(**env):
        local["kubectl"]["apply", "-f", "-"].run(stdin=manifest)


def install_corbusier_chart(cfg: Config, env: dict[str, str]) -> None:
    """Install or upgrade the Corbusier chart using local preview values."""
    with local.env(**env):
        helm = local["helm"]
        helm[
            "upgrade",
            "--install",
            cfg.app_name,
            str(cfg.chart_path),
            "--namespace",
            cfg.namespace,
            "--create-namespace",
            "--values",
            str(cfg.values_file),
            "--set",
            f"image.repository={cfg.image_repo}",
            "--set",
            f"image.tag={cfg.image_tag}",
            "--set",
            f"existingSecretName={cfg.app_secret_name}",
            "--wait",
            "--timeout",
            "600s",
        ] & FG


def print_status(cfg: Config, env: dict[str, str]) -> None:
    """Print pod, service, and ingress status for the preview namespace."""
    with local.env(**env):
        kubectl = local["kubectl"]
        kubectl["get", "pods", f"--namespace={cfg.namespace}", "-o", "wide"] & FG
        kubectl["get", "service,ingress", f"--namespace={cfg.namespace}"] & FG


def tail_logs(cfg: Config, env: dict[str, str], *, follow: bool = False) -> None:
    """Tail application logs from Corbusier pods."""
    with local.env(**env):
        command = local["kubectl"][
            "logs",
            f"--selector=app.kubernetes.io/name={cfg.app_name}",
            f"--namespace={cfg.namespace}",
        ]
        if follow:
            command = command["--follow"]
        command & FG
