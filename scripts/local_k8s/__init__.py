"""Local k3d preview helpers for Corbusier.

This package groups the local preview utilities that back the
`scripts/local_k8s.py` CLI entry point. It provides orchestration helpers,
Kubernetes and Helm wrappers, cluster lifecycle utilities, and validation code
for bringing up the Corbusier preview environment on a developer workstation.

The helpers expect a local toolchain with `k3d`, `kubectl`, `helm`, and, when
building images, `docker` available in `PATH`. They are designed to be called
either indirectly through the CLI or directly from repository-local Python
automation.

Examples
--------
Run the preview environment from the command line:

>>> # uv run scripts/local_k8s.py up

Import the orchestration layer directly:

>>> from local_k8s.orchestration import setup_environment
>>> setup_environment("corbusier-local", "corbusier", None, skip_build=True)
0
"""
