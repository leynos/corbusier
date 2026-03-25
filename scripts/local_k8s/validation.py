"""Validation helpers for the local k3d workflow."""

from __future__ import annotations

import base64
import shutil
import socket


class LocalK8sError(Exception):
    """Base error for local-k8s workflow failures."""


class ExecutableNotFoundError(LocalK8sError):
    """Raised when a required CLI tool is unavailable."""


class PortMismatchError(LocalK8sError):
    """Raised when a requested ingress port conflicts with an existing cluster."""


class SecretDecodeError(LocalK8sError):
    """Raised when a Kubernetes Secret field cannot be decoded."""


def require_exe(name: str) -> None:
    """Ensure a required executable is available."""
    if shutil.which(name) is None:
        raise ExecutableNotFoundError(f"Required executable '{name}' not found in PATH")


def pick_free_loopback_port() -> int:
    """Pick a currently unused loopback port."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    if not isinstance(port, int):
        raise LocalK8sError("Failed to allocate an ingress port")
    return port


def b64decode_k8s_secret_field(value: str) -> str:
    """Decode a base64-encoded Secret field into UTF-8 text."""
    try:
        return base64.b64decode(value).decode("utf-8")
    except (ValueError, UnicodeDecodeError) as error:
        raise SecretDecodeError(f"Failed to decode Kubernetes Secret field: {error}") from error
