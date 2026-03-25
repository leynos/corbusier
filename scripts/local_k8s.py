#!/usr/bin/env -S uv run python
# /// script
# requires-python = ">=3.13"
# dependencies = ["cyclopts>=2.9", "plumbum>=1.8"]
# ///

"""Manage a local k3d preview environment for Corbusier."""

from __future__ import annotations

import functools
import sys
from typing import Annotated, Callable, ParamSpec, TypeVar

from cyclopts import App, Parameter
from plumbum.commands.processes import ProcessExecutionError

from local_k8s.orchestration import (
    setup_environment,
    show_environment_status,
    stream_environment_logs,
    teardown_environment,
)
from local_k8s.validation import LocalK8sError


P = ParamSpec("P")
R = TypeVar("R")


def handle_cli_errors(func: Callable[P, int]) -> Callable[P, int]:
    """Normalize CLI exceptions into user-facing errors and exit codes."""

    @functools.wraps(func)
    def wrapper(*args: P.args, **kwargs: P.kwargs) -> int:
        try:
            return int(func(*args, **kwargs))
        except LocalK8sError as error:
            print(f"Error: {error}", file=sys.stderr)
            return 1
        except ProcessExecutionError as error:
            print(f"Error: command failed: {error.argv}", file=sys.stderr)
            if error.stderr:
                print(error.stderr, file=sys.stderr)
            return 1

    return wrapper


app = App(name="local_k8s", help="Local k3d preview environment for Corbusier", version="0.1.0")


@app.command
@handle_cli_errors
def up(
    *,
    cluster_name: Annotated[str, Parameter(env_var="CORBUSIER_K3D_CLUSTER")] = "corbusier-local",
    namespace: Annotated[str, Parameter(env_var="CORBUSIER_K3D_NAMESPACE")] = "corbusier",
    ingress_port: Annotated[int | None, Parameter(env_var="CORBUSIER_K3D_PORT")] = None,
    skip_build: bool = False,
) -> int:
    """Create or update the local preview environment."""
    return setup_environment(cluster_name, namespace, ingress_port, skip_build=skip_build)


@app.command
@handle_cli_errors
def down(
    *,
    cluster_name: Annotated[str, Parameter(env_var="CORBUSIER_K3D_CLUSTER")] = "corbusier-local",
) -> int:
    """Delete the local preview cluster."""
    return teardown_environment(cluster_name)


@app.command
@handle_cli_errors
def status(
    *,
    cluster_name: Annotated[str, Parameter(env_var="CORBUSIER_K3D_CLUSTER")] = "corbusier-local",
    namespace: Annotated[str, Parameter(env_var="CORBUSIER_K3D_NAMESPACE")] = "corbusier",
) -> int:
    """Show local preview status."""
    return show_environment_status(cluster_name, namespace)


@app.command
@handle_cli_errors
def logs(
    *,
    cluster_name: Annotated[str, Parameter(env_var="CORBUSIER_K3D_CLUSTER")] = "corbusier-local",
    namespace: Annotated[str, Parameter(env_var="CORBUSIER_K3D_NAMESPACE")] = "corbusier",
    follow: bool = False,
) -> int:
    """Tail logs from the local preview environment."""
    return stream_environment_logs(cluster_name, namespace, follow=follow)


def main() -> int:
    """Run the CLI application."""
    result = app()
    return int(result) if result is not None else 0


if __name__ == "__main__":
    sys.exit(main())
