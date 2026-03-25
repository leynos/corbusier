"""Deployment helpers for the local Corbusier preview environment.

This module contains the imperative deployment actions used by the local
preview orchestration layer. It builds the local Docker image, creates the
runtime Kubernetes `Secret`, installs the Corbusier Helm chart, reports
deployment status, and tails application logs from the preview namespace.

Typical usage flows through `local_k8s.orchestration`, but the functions remain
small and composable so repository-local scripts can reuse them directly when a
single deployment step needs to be repeated.

Examples
--------
Build and import the local Corbusier image as part of a preview refresh:

>>> # build_docker_image(cfg)

Tail logs from the Helm release after deployment:

>>> # tail_logs(cfg, env, follow=True)
"""

from __future__ import annotations

import json

from plumbum import FG, local

from local_k8s.config import Config


def build_docker_image(cfg: Config) -> None:
    """Build the Corbusier image from the repository root.

    Parameters
    ----------
    cfg : Config
        Local preview configuration containing the project root and image tag.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when the Docker build command fails.
    """
    with local.cwd(cfg.project_root):
        local["docker"]["build", "-t", f"{cfg.image_repo}:{cfg.image_tag}", "."] & FG


def create_app_secret(cfg: Config, env: dict[str, str], database_url: str, valkey_url: str) -> None:
    """Create or update the application Secret used by the Helm chart.

    Parameters
    ----------
    cfg : Config
        Local preview configuration describing the application `Secret` name
        and target namespace.
    env : dict[str, str]
        Environment variables used to target the correct Kubernetes cluster.
    database_url : str
        Postgres connection string to store in the application `Secret`.
    valkey_url : str
        Valkey connection string to store in the application `Secret`.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when `kubectl apply` fails.
    """
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
    """Install or upgrade the Corbusier chart using local preview values.

    Parameters
    ----------
    cfg : Config
        Local preview configuration describing the chart path, release name,
        image, and application `Secret`.
    env : dict[str, str]
        Environment variables used to target the correct Kubernetes cluster.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when the Helm install or upgrade command fails.
    """
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
    """Print pod, service, and ingress status for the preview namespace.

    Parameters
    ----------
    cfg : Config
        Local preview configuration describing the target namespace.
    env : dict[str, str]
        Environment variables used to target the correct Kubernetes cluster.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when `kubectl get` fails.
    """
    with local.env(**env):
        kubectl = local["kubectl"]
        kubectl["get", "pods", f"--namespace={cfg.namespace}", "-o", "wide"] & FG
        kubectl["get", "service,ingress", f"--namespace={cfg.namespace}"] & FG


def tail_logs(cfg: Config, env: dict[str, str], *, follow: bool = False) -> None:
    """Tail application logs from Corbusier pods.

    Parameters
    ----------
    cfg : Config
        Local preview configuration describing the Helm release and namespace.
    env : dict[str, str]
        Environment variables used to target the correct Kubernetes cluster.
    follow : bool, default=False
        When `True`, stream logs until interrupted. When `False`, print the
        current log buffer and return.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Raised when `kubectl logs` fails.
    """
    with local.env(**env):
        command = local["kubectl"][
            "logs",
            f"--selector=app.kubernetes.io/instance={cfg.app_name}",
            f"--namespace={cfg.namespace}",
        ]
        if follow:
            command = command["--follow"]
        command & FG
