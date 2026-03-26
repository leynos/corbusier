"""Configuration for Corbusier's local k3d preview environment."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parents[2]


@dataclass(frozen=True, slots=True)
class Config:
    """Configuration for the local preview workflow."""

    cluster_name: str = "corbusier-local"
    namespace: str = "corbusier"
    app_name: str = "corbusier"
    ingress_port: int | None = None
    project_root: Path = PROJECT_ROOT
    chart_path: Path = PROJECT_ROOT / "charts" / "corbusier"
    image_repo: str = "corbusier"
    image_tag: str = "local"
    cnpg_release: str = "cnpg"
    cnpg_namespace: str = "cnpg-system"
    valkey_release: str = "valkey-operator"
    valkey_namespace: str = "valkey-operator-system"
    values_file: Path = PROJECT_ROOT / "charts" / "corbusier" / "values.local.yaml"
    pg_cluster_name: str = "pg-corbusier"
    valkey_name: str = "valkey-corbusier"
    app_secret_name: str = "corbusier"
