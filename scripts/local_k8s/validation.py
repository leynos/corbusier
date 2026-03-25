"""Validation helpers for the local `k3d` workflow.

This module centralises the low-level validation and decoding logic shared by
the local preview scripts. It checks that required executables are present,
allocates loopback ports for ingress exposure, and decodes Secret values read
from Kubernetes resources.

The helpers are intentionally small so they can be reused across the
orchestration, Kubernetes, and operator-specific modules without duplicating
error handling or user-facing failure messages.

Examples
--------
Validate that required tools are available before provisioning a cluster:

>>> require_exe("k3d")

Decode a base64-encoded Secret field:

>>> b64decode_k8s_secret_field("b2s=")
'ok'
"""

from __future__ import annotations

import base64
import binascii
import shutil
import socket


class LocalK8sError(Exception):
    """Base error for local-k8s workflow failures.

    Attributes
    ----------
    args : tuple[object, ...]
        Standard exception arguments describing the failure.
    """


class ExecutableNotFoundError(LocalK8sError):
    """Raised when a required CLI tool is unavailable.

    Attributes
    ----------
    args : tuple[object, ...]
        Standard exception arguments describing the missing executable.
    """


class PortMismatchError(LocalK8sError):
    """Raised when a requested ingress port conflicts with an existing cluster.

    Attributes
    ----------
    args : tuple[object, ...]
        Standard exception arguments describing the conflicting ports.
    """


class SecretDecodeError(LocalK8sError):
    """Raised when a Kubernetes Secret field cannot be decoded.

    Attributes
    ----------
    args : tuple[object, ...]
        Standard exception arguments describing the decode failure.
    """


class LocalK8sSecretError(LocalK8sError):
    """Raised when a Kubernetes Secret is missing required fields or values.

    Attributes
    ----------
    args : tuple[object, ...]
        Standard exception arguments describing the invalid Secret content.
    """


def require_exe(name: str) -> None:
    """Ensure a required executable is available.

    Parameters
    ----------
    name : str
        Executable name to locate in `PATH`.

    Returns
    -------
    None
        This function is called for its side effects.

    Raises
    ------
    ExecutableNotFoundError
        Raised when the executable cannot be found in `PATH`.
    """
    if shutil.which(name) is None:
        raise ExecutableNotFoundError(f"Required executable '{name}' not found in PATH")


def pick_free_loopback_port() -> int:
    """Pick a currently unused loopback port.

    Returns
    -------
    int
        Free TCP port bound on `127.0.0.1`.

    Raises
    ------
    LocalK8sError
        Raised when the socket API returns a non-integer port value.
    OSError
        Raised when the loopback socket cannot be opened or bound.
    """
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    if not isinstance(port, int):
        raise LocalK8sError("Failed to allocate an ingress port")
    return port


def b64decode_k8s_secret_field(value: str) -> str:
    """Decode a base64-encoded Secret field into UTF-8 text.

    Parameters
    ----------
    value : str
        Base64-encoded Secret field value.

    Returns
    -------
    str
        UTF-8 decoded plaintext value.

    Raises
    ------
    SecretDecodeError
        Raised when the input is not valid base64 or cannot be decoded as
        UTF-8 text.
    """
    try:
        return base64.b64decode(value, validate=True).decode("utf-8")
    except (binascii.Error, ValueError, UnicodeDecodeError) as error:
        raise SecretDecodeError(f"Failed to decode Kubernetes Secret field: {error}") from error
