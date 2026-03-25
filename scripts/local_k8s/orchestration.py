"""High-level orchestration for the local preview workflow."""

from __future__ import annotations

from plumbum import local

from local_k8s.cnpg import create_cnpg_cluster, install_cnpg_operator, read_pg_app_uri, wait_for_cnpg_ready
from local_k8s.config import Config
from local_k8s.deployment import build_docker_image, create_app_secret, install_corbusier_chart, print_status, tail_logs
from local_k8s.k3d import cluster_exists, create_k3d_cluster, delete_k3d_cluster, get_cluster_ingress_port, import_image_to_k3d, kubeconfig_env
from local_k8s.k8s import ensure_namespace
from local_k8s.validation import LocalK8sError, PortMismatchError, pick_free_loopback_port, require_exe
from local_k8s.valkey import create_valkey_instance, install_valkey_operator, read_valkey_uri, wait_for_valkey_ready


def _require_tools(skip_build: bool) -> None:
    """Ensure required executables are available."""
    for executable in ("k3d", "kubectl", "helm"):
        require_exe(executable)
    if not skip_build:
        require_exe("docker")


def _ensure_cluster(cluster_name: str, ingress_port: int | None) -> int:
    """Create or reuse the k3d cluster and return the ingress port."""
    if cluster_exists(cluster_name):
        existing_port = get_cluster_ingress_port(cluster_name)
        if existing_port is None:
            raise LocalK8sError(
                f"Could not determine the ingress port for existing cluster '{cluster_name}'"
            )
        if ingress_port is not None and ingress_port != existing_port:
            raise PortMismatchError(
                f"Cluster '{cluster_name}' already uses ingress port {existing_port}, "
                f"not requested port {ingress_port}"
            )
        print(f"Reusing existing k3d cluster '{cluster_name}' on port {existing_port}...")
        return existing_port

    selected_port = ingress_port or pick_free_loopback_port()
    print(f"Creating k3d cluster '{cluster_name}' on port {selected_port}...")
    create_k3d_cluster(cluster_name, selected_port)
    return selected_port


def _print_success_banner(port: int) -> None:
    """Print connection details after a successful deployment."""
    print()
    print("=" * 60)
    print("Corbusier preview environment ready")
    print(f"Preview URL: http://127.0.0.1:{port}/")
    print(f"Health URL:  http://127.0.0.1:{port}/health/live")
    print("Status:      make local-k8s-status")
    print("Logs:        make local-k8s-logs")
    print("Down:        make local-k8s-down")
    print("=" * 60)


def setup_environment(cluster_name: str, namespace: str, ingress_port: int | None, *, skip_build: bool) -> int:
    """Provision platform services and deploy Corbusier into k3d."""
    _require_tools(skip_build)

    port = _ensure_cluster(cluster_name, ingress_port)
    cfg = Config(cluster_name=cluster_name, namespace=namespace, ingress_port=port)
    env = kubeconfig_env(cfg.cluster_name)

    print("Ensuring application namespace exists...")
    ensure_namespace(cfg.namespace, env)

    print("Installing CloudNativePG operator...")
    install_cnpg_operator(cfg, env)
    print("Creating CloudNativePG cluster...")
    create_cnpg_cluster(cfg, env)
    print("Waiting for Postgres to become ready...")
    wait_for_cnpg_ready(cfg, env)

    print("Installing Valkey operator...")
    install_valkey_operator(cfg, env)
    print("Creating Valkey instance...")
    create_valkey_instance(cfg, env)
    print("Waiting for Valkey to become ready...")
    wait_for_valkey_ready(cfg, env)

    print("Reading operator-managed connection details...")
    database_url = read_pg_app_uri(cfg, env)
    valkey_url = read_valkey_uri(cfg, env)

    print("Creating Corbusier application Secret...")
    create_app_secret(cfg, env, database_url, valkey_url)

    if skip_build:
        print("Skipping Docker build (--skip-build)")
    else:
        print(f"Building Docker image {cfg.image_repo}:{cfg.image_tag}...")
        build_docker_image(cfg)
        print("Importing image into the k3d cluster...")
        import_image_to_k3d(cfg.cluster_name, cfg.image_repo, cfg.image_tag)

    print("Installing Corbusier Helm chart...")
    install_corbusier_chart(cfg, env)
    _print_success_banner(port)
    return 0


def teardown_environment(cluster_name: str) -> int:
    """Delete the local k3d cluster."""
    require_exe("k3d")
    if not cluster_exists(cluster_name):
        print(f"Cluster '{cluster_name}' does not exist.")
        return 0
    print(f"Deleting k3d cluster '{cluster_name}'...")
    delete_k3d_cluster(cluster_name)
    return 0


def show_environment_status(cluster_name: str, namespace: str) -> int:
    """Show current pod and ingress status for the preview environment."""
    require_exe("kubectl")
    if not cluster_exists(cluster_name):
        print(f"Cluster '{cluster_name}' does not exist.")
        return 1
    cfg = Config(cluster_name=cluster_name, namespace=namespace)
    env = kubeconfig_env(cluster_name)
    print_status(cfg, env)
    return 0


def stream_environment_logs(cluster_name: str, namespace: str, *, follow: bool) -> int:
    """Stream logs from Corbusier pods in the preview namespace."""
    require_exe("kubectl")
    if not cluster_exists(cluster_name):
        print(f"Cluster '{cluster_name}' does not exist.")
        return 1
    cfg = Config(cluster_name=cluster_name, namespace=namespace)
    env = kubeconfig_env(cluster_name)
    tail_logs(cfg, env, follow=follow)
    return 0
