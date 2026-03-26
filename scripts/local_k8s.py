#!/usr/bin/env -S uv run python
# /// script
# requires-python = ">=3.13"
# dependencies = ["cyclopts>=2.9", "plumbum>=1.8"]
# ///

"""Command-line interface for the local Corbusier k3d preview workflow.

This module exposes the repository-facing CLI used by `make local-k8s-*`
targets. It maps command-line arguments and environment variables onto the
local preview orchestration layer, normalises operational failures into
user-facing exit codes, and keeps the command defaults aligned with
`local_k8s.config.Config`.

The CLI is intended for local developer use from the repository root:

- `up` provisions or reuses a `k3d` cluster and deploys Corbusier.
- `down` tears down the local preview cluster.
- `status` reports the current Kubernetes resource status.
- `logs` tails application logs from the preview namespace.

Examples
--------
Start the preview environment using the default cluster and namespace:

>>> # uv run scripts/local_k8s.py up

Inspect the current preview deployment in a custom namespace:

>>> # uv run scripts/local_k8s.py status --namespace demo
"""

from __future__ import annotations

from collections.abc import Callable
import functools
import sys
from typing import Annotated

from cyclopts import App, Parameter
from plumbum.commands.processes import ProcessExecutionError

from local_k8s.config import Config
from local_k8s.orchestration import (
    setup_environment,
    show_environment_status,
    stream_environment_logs,
    teardown_environment,
)
from local_k8s.validation import LocalK8sError


DEFAULT_CONFIG = Config()


def handle_cli_errors(func: Callable[..., int]) -> Callable[..., int]:
    """Normalise CLI exceptions into user-facing errors and exit codes.

    Parameters
    ----------
    func : collections.abc.Callable[..., int]
        CLI command function that returns a process-style exit code.

    Returns
    -------
    collections.abc.Callable[..., int]
        Wrapped command function that prints normalised error messages to
        standard error and returns a non-zero exit code on handled failures.

    Raises
    ------
    plumbum.commands.processes.ProcessExecutionError
        Re-raised only for command failures not intercepted by the wrapper.
    Exception
        Re-raised for unexpected exceptions so debugging information is not
        hidden.

    Examples
    --------
    Wrap a command so `LocalK8sError` values map to exit code `1`:

    >>> @handle_cli_errors
    ... def command() -> int:
    ...     return 0
    """

    @functools.wraps(func)
    def wrapper(*args: object, **kwargs: object) -> int:
        try:
            return func(*args, **kwargs)
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
    cluster_name: Annotated[str, Parameter(env_var="CORBUSIER_K3D_CLUSTER")] = DEFAULT_CONFIG.cluster_name,
    namespace: Annotated[str, Parameter(env_var="CORBUSIER_K3D_NAMESPACE")] = DEFAULT_CONFIG.namespace,
    ingress_port: Annotated[int | None, Parameter(env_var="CORBUSIER_K3D_PORT")] = None,
    skip_build: bool = False,
) -> int:
    """Create or update the local preview environment.

    Parameters
    ----------
    cluster_name : str
        Name of the `k3d` cluster to create or reuse.
    namespace : str
        Namespace where Corbusier and its dependencies should be deployed.
    ingress_port : int | None
        Optional loopback ingress port override. When `None`, the orchestration
        layer selects a candidate port.
    skip_build : bool
        When `True`, skip the local Docker build and image-import steps.

    Returns
    -------
    int
        Process-style exit code. `0` indicates success.

    Raises
    ------
    local_k8s.validation.LocalK8sError
        Raised before decoration when preview validation fails.
    plumbum.commands.processes.ProcessExecutionError
        Raised before decoration when an external command fails.

    Examples
    --------
    Start the default preview environment:

    >>> # uv run scripts/local_k8s.py up
    """
    return setup_environment(cluster_name, namespace, ingress_port, skip_build=skip_build)


@app.command
@handle_cli_errors
def down(
    *,
    cluster_name: Annotated[str, Parameter(env_var="CORBUSIER_K3D_CLUSTER")] = DEFAULT_CONFIG.cluster_name,
) -> int:
    """Delete the local preview cluster.

    Parameters
    ----------
    cluster_name : str
        Name of the `k3d` cluster to delete.

    Returns
    -------
    int
        Process-style exit code. `0` indicates success or that the cluster was
        already absent.

    Raises
    ------
    local_k8s.validation.LocalK8sError
        Raised before decoration when validation fails.
    plumbum.commands.processes.ProcessExecutionError
        Raised before decoration when cluster deletion fails.

    Examples
    --------
    Delete the default preview cluster:

    >>> # uv run scripts/local_k8s.py down
    """
    return teardown_environment(cluster_name)


@app.command
@handle_cli_errors
def status(
    *,
    cluster_name: Annotated[str, Parameter(env_var="CORBUSIER_K3D_CLUSTER")] = DEFAULT_CONFIG.cluster_name,
    namespace: Annotated[str, Parameter(env_var="CORBUSIER_K3D_NAMESPACE")] = DEFAULT_CONFIG.namespace,
) -> int:
    """Show local preview status.

    Parameters
    ----------
    cluster_name : str
        Name of the `k3d` cluster to inspect.
    namespace : str
        Namespace containing the preview deployment.

    Returns
    -------
    int
        Process-style exit code. `0` indicates success.

    Raises
    ------
    local_k8s.validation.LocalK8sError
        Raised before decoration when validation fails.
    plumbum.commands.processes.ProcessExecutionError
        Raised before decoration when status queries fail.

    Examples
    --------
    Print the preview resource status:

    >>> # uv run scripts/local_k8s.py status
    """
    return show_environment_status(cluster_name, namespace)


@app.command
@handle_cli_errors
def logs(
    *,
    cluster_name: Annotated[str, Parameter(env_var="CORBUSIER_K3D_CLUSTER")] = DEFAULT_CONFIG.cluster_name,
    namespace: Annotated[str, Parameter(env_var="CORBUSIER_K3D_NAMESPACE")] = DEFAULT_CONFIG.namespace,
    follow: bool = False,
) -> int:
    """Tail logs from the local preview environment.

    Parameters
    ----------
    cluster_name : str
        Name of the `k3d` cluster to inspect.
    namespace : str
        Namespace containing the preview deployment.
    follow : bool
        When `True`, continue streaming logs until interrupted.

    Returns
    -------
    int
        Process-style exit code. `0` indicates success.

    Raises
    ------
    local_k8s.validation.LocalK8sError
        Raised before decoration when validation fails.
    plumbum.commands.processes.ProcessExecutionError
        Raised before decoration when log retrieval fails.

    Examples
    --------
    Follow logs from the preview deployment:

    >>> # uv run scripts/local_k8s.py logs --follow
    """
    return stream_environment_logs(cluster_name, namespace, follow=follow)


def main() -> int:
    """Run the local preview CLI application.

    Parameters
    ----------
    None
        This entry point accepts arguments from `sys.argv` via Cyclopts.

    Returns
    -------
    int
        Process-style exit code returned by the selected CLI command, or `0`
        when Cyclopts returns `None`.

    Raises
    ------
    SystemExit
        Raised indirectly by the module entry point when `sys.exit(main())`
        executes.

    Examples
    --------
    Run the script as a module entry point:

    >>> # uv run scripts/local_k8s.py up
    """
    result = app()
    return result if result is not None else 0


if __name__ == "__main__":
    sys.exit(main())
