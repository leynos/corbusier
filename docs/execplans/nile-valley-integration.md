# Local k3d preview and Nile Valley integration for Corbusier

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: IN PROGRESS

## Purpose / big picture

Corbusier is an AI agent orchestration platform written in Rust. It depends on
PostgreSQL (via Diesel ORM) and will eventually depend on Valkey for caching.
Today there is no container image, no Helm chart, and no local Kubernetes
workflow.

After this work, a developer can run a single command (`make local-k8s-up`) to
spin up a k3d cluster containing Corbusier backed by a real Postgres instance
(provisioned by CloudNativePG) and a Valkey instance (provisioned by the Valkey
operator). The same Helm chart deploys unchanged into the Nile Valley
platform's GitOps pipeline for ephemeral previews and production.

Observable success: `make local-k8s-up` prints a preview URL; `curl`-ing that
URL's health endpoint returns HTTP 200; `make local-k8s-status` shows ready
pods; `make local-k8s-down` tears the cluster down cleanly.

## Constraints

- The Helm chart must deploy only the Corbusier application. Platform
  services (CloudNativePG, Valkey operator, Traefik, cert-manager, ExternalDNS,
  Vault, External Secrets Operator) live in the Nile Valley platform repository
  and must not be bundled into the app chart.
- The chart must be compatible with FluxCD HelmRelease and Kustomize
  overlays, matching the pattern established by the Nile Valley
  `deploy/charts/example-app/` reference chart.
- Scripts must follow the scripting standards in
  `docs/scripting-standards.md`: Python 3.13, `uv` shebang, Cyclopts CLI,
  `plumbum` for subprocess execution.
- The Dockerfile must produce a multi-stage build targeting a minimal
  runtime image with a non-root user.
- Existing Rust source, tests, and Makefile targets (`make test`,
  `make lint`, `make check-fmt`) must continue to pass without modification.
- The Nile Valley platform repository (`../../nile-valley/`) must not be
  modified by this plan.
- Markdown files must pass `make markdownlint` and follow the
  documentation style guide (`docs/documentation-style-guide.md`).

## Tolerances (exception triggers)

- Scope: if implementation requires modifying more than 5 existing Rust
  source files, stop and escalate.
- Interface: if any existing public Rust API signature must change, stop
  and escalate.
- Dependencies: if a new Rust crate dependency is required (beyond what is
  already in `Cargo.toml`), stop and escalate.
- Iterations: if Helm template rendering or Docker build fails after 3
  attempts, stop and escalate.
- Ambiguity: if Corbusier's runtime entrypoint requirements conflict with
  the container image design, stop and present options.

## Risks

- Risk: Corbusier has no HTTP server or health endpoint yet (`main.rs` is
  a stub printing "Hello from Corbusier!"). The Helm chart needs a container
  that runs and stays alive for Kubernetes probes. Severity: high Likelihood:
  certain Mitigation: Stage B introduces a minimal HTTP health endpoint using a
  lightweight Rust HTTP server (e.g., `axum` or `hyper`) behind a feature flag
  or as a separate binary target. Alternatively, the initial deployment can use
  a long-running sleep loop with liveness based on process existence, deferring
  the health endpoint to a follow-up. The decision is recorded when Stage B
  begins.

- Risk: Diesel requires `libpq` at compile time for the `postgres`
  feature. The Dockerfile must include `libpq-dev` in the build stage and
  `libpq5` in the runtime stage. Severity: medium Likelihood: certain
  Mitigation: Document the exact Debian packages in the Dockerfile sketch and
  verify in Stage C.

- Risk: The Valkey operator CRD names and secret field names may differ
  from Ghillie's assumptions. Severity: low Likelihood: medium Mitigation: The
  local k8s script will read connection details from the operator-generated
  secret, matching Ghillie's proven approach. Confirm field names during Stage
  D.

- Risk: Corbusier's `pg-embed-setup-unpriv` dependency (used for test
  Postgres) may conflict with the production Diesel migration workflow.
  Severity: low Likelihood: low Mitigation: The Dockerfile only builds the
  release binary; test-only dependencies are excluded via `--release` and
  feature flags.

## Progress

- [x] Stage A: Research and design (no code changes).
- [x] Stage B: Runtime entrypoint and Dockerfile.
- [x] Stage C: Helm chart.
- [x] Stage D: Local k3d lifecycle script.
- [x] Stage E: Makefile targets, documentation, and validation.

## Surprises & discoveries

- Stage B had already landed as two earlier commits on this branch
  (`fb84262` and `9fc20f1`), so the remaining work was to finish the chart,
  local-k8s workflow, and documentation rather than to replace a stub runtime.
- The current Nile Valley `example-app` reference does not ship an
  `ExternalSecret` template in the inspected checkout, so the Corbusier chart
  had to combine Nile Valley security-context conventions with Ghillie's
  External Secrets pattern.
- The environment used for implementation does not currently expose all local
  preview tools (`helm`, `docker`, `k3d`, and `kubectl`) at once, so end-to-end
  local-cluster validation may need to be completed on a machine with the full
  Kubernetes toolchain installed.

## Decision log

- Decision: keep the Actix-based health endpoint and Dockerfile already on the
  branch instead of reworking Stage B. Rationale: the implementation already
  satisfied the plan's runtime contract and keeping it avoided unnecessary
  churn.
- Decision: use `plumbum` directly in the Corbusier local-k8s workflow rather
  than porting Ghillie's `subprocess` helpers verbatim. Rationale: Corbusier's
  scripting standards explicitly require `plumbum`.
- Decision: model ingress values as `ingress.hosts` and `ingress.tls` rather
  than a single `hostname` field. Rationale: the list-based structure better
  matches Helm/GitOps overlay patterns and the local hostless-ingress use case.

## Outcomes & retrospective

- Corbusier now has a Kubernetes-ready runtime image, a Helm chart shaped for
  local preview and Nile Valley overlays, and a local `k3d` lifecycle script
  exposed through Make targets.
- The implementation preserves the repository's existing Rust quality gates and
  keeps platform operators out of the application chart.
- Remaining follow-up risk is primarily environmental verification: the code
  path is in place, but full `k3d` integration testing depends on a machine
  that has the complete local Kubernetes toolchain available.

## Context and orientation

### Repository layout (Corbusier)

Corbusier is a single-crate Rust workspace at the repository root. Key paths:

- `Cargo.toml` — workspace and crate definition; depends on Diesel
  (Postgres), Tokio, serde, chrono, uuid, tracing, and others.
- `src/main.rs` — stub entry point (prints a greeting and exits).
- `src/lib.rs` — library root with modules for agent backends, context,
  hooks, messages, tasks, tenants, and tool registry.
- `Makefile` — build, test, lint, and format targets.
- `docs/` — design documents, roadmap, and scripting standards.
- `docs/execplans/` — execution plans (this file lives here).

There is no `Dockerfile`, no `charts/` directory, no `scripts/local_k8s`
package, and no `values.yaml` today.

### Reference implementation (Ghillie)

The Ghillie repository (`../../ghillie/`) has a complete local k3d preview
workflow that serves as the template for this plan:

- `charts/ghillie/` — Helm chart with Deployment, Service, Ingress,
  ConfigMap, ExternalSecret, and ServiceAccount templates.
- `scripts/local_k8s.py` — Cyclopts CLI entry point with `up`, `down`,
  `status`, `logs` subcommands.
- `scripts/local_k8s/` — Python package with modules for config, k3d,
  k8s, cnpg, valkey, deployment, orchestration, and validation.
- `docs/local-k8s-preview-design.md` — design document describing the
  Helm chart, Dockerfile, and lifecycle script.

### Nile Valley platform

The Nile Valley platform repository (`../../nile-valley/`) provides:

- `deploy/charts/example-app/` — reference Helm chart for applications
  deployed on the platform. Uses ConfigMap for non-secret config,
  `existingSecretName` for secrets, hardened security contexts, health probes,
  PDB, HPA, and topology spread constraints.
- `infra/modules/` — Terraform/OpenTofu modules for platform services
  (CNPG, Valkey, Traefik, cert-manager, ExternalDNS, Vault, ESO).
- `scripts/` — Python scripts for cluster provisioning, GitOps manifest
  rendering, and Vault bootstrapping.

The Corbusier Helm chart must align with the `example-app` chart's conventions
(security context, probe paths, config/secret patterns, label scheme) so it
deploys cleanly on the Nile Valley platform.

## Plan of work

### Stage A: Research and design (no code changes)

This stage is complete once the ExecPlan is approved. No repository changes are
made.

Review the following to inform the design:

1. Ghillie's Helm chart (`charts/ghillie/`), values, and templates.
2. Ghillie's local k8s script package (`scripts/local_k8s/`).
3. Nile Valley's `example-app` chart for conventions (probes, security
   context, labels, secret binding).
4. Corbusier's `Cargo.toml` for runtime dependencies affecting the
   Dockerfile.
5. Corbusier's scripting standards (`docs/scripting-standards.md`).

### Stage B: Runtime entrypoint and Dockerfile

Corbusier previously lacked a long-running process, so this stage introduced an
HTTP server that exposes `/health/live` and `/health/ready` plus a multi-stage
Dockerfile.

**B.1: Define a health port in the domain layer.**

Corbusier follows hexagonal architecture (see `src/lib.rs`). The HTTP server is
an infrastructure adapter; health-check logic belongs behind a port trait so
the domain remains free of web-framework dependencies.

Create `src/health/mod.rs` (a new domain module) containing:

- A `HealthStatus` enum (`Healthy`, `Degraded`, `Unhealthy`) — the
  domain's vocabulary for readiness.
- A `HealthCheck` trait (port):

  ```rust
  /// Port for health observation.
  ///
  /// Adapters (HTTP, gRPC, CLI) call this to obtain the
  /// application's liveness and readiness status.
  pub trait HealthCheck: Send + Sync {
      /// Report whether the process is alive.
      fn liveness(&self) -> HealthStatus;
      /// Report whether the process is ready to serve traffic.
      fn readiness(&self) -> HealthStatus;
  }
  ```

- A `SimpleHealthCheck` struct that implements the trait, always
  returning `Healthy`. This is sufficient for the initial deployment; future
  milestones can inject real dependency probes (Postgres connectivity, Valkey
  ping) via the same port.

Register the module in `src/lib.rs` (`pub mod health;`).

**B.2: Add an Actix Web HTTP adapter.**

Create `src/health/actix_adapter.rs` — the driving adapter that wires the
`HealthCheck` port to HTTP. This module:

- Accepts an `Arc<dyn HealthCheck>` as Actix Web application data.
- Defines handler functions for `GET /health/live` and
  `GET /health/ready` that delegate to the port trait methods and map
  `HealthStatus` to HTTP status codes (200 for `Healthy`, 503 for
  `Degraded`/`Unhealthy`).
- Exposes a `pub fn health_routes(cfg: &mut web::ServiceConfig)` that
  registers both routes, keeping the adapter self-contained.

This keeps Actix Web types confined to the adapter; the domain module has no
dependency on `actix-web`.

**B.3: Update the application entry point.**

In `src/main.rs`, replace the stub with an `#[actix_web::main]` entry point
that:

- Constructs a `SimpleHealthCheck` and wraps it in `Arc<dyn HealthCheck>`.
- Starts an `HttpServer` listening on port 8080 (configurable via
  `CORBUSIER_PORT` env var).
- Registers `health_routes` on the Actix app.
- Logs startup with `tracing`.

This requires adding `actix-web` as a dependency to `Cargo.toml`. That
constitutes a new dependency and falls within the tolerance for dependencies if
the user approves during the approval gate. If not approved, the fallback is a
process that sleeps indefinitely and relies on process-existence liveness
probes (no HTTP health check).

**B.4: Create the Dockerfile.**

Create `Dockerfile` at the repository root:

```dockerfile
FROM rust:1.87-slim-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq-dev pkg-config && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release --bin corbusier

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*
RUN groupadd -r corbusier && useradd -r -g corbusier corbusier
COPY --from=build /build/target/release/corbusier /usr/local/bin/corbusier
USER corbusier
EXPOSE 8080
ENTRYPOINT ["corbusier"]
```

**B.5: Create `.dockerignore`.**

```plaintext
target/
.git/
docs/
tests/
*.md
```

**Validation for Stage B:**

```bash
docker build -t corbusier:local .
docker run --rm -p 8080:8080 corbusier:local &
curl http://127.0.0.1:8080/health/live
# Expected: 200 OK, body "ok"
docker stop $(docker ps -q --filter ancestor=corbusier:local)
```

Run `make test && make lint && make check-fmt` to confirm existing code is
unaffected.

### Stage C: Helm chart

Create `charts/corbusier/` modelled on both the Ghillie chart and the Nile
Valley `example-app` chart. The chart must satisfy conventions from both.

**C.1: Chart metadata (`charts/corbusier/Chart.yaml`).**

```yaml
apiVersion: v2
name: corbusier
description: Corbusier AI agent orchestration platform
type: application
version: 0.1.0
appVersion: "0.1.0"
maintainers:
  - name: Corbusier Maintainers
```

**C.2: Values interface (`charts/corbusier/values.yaml`).**

Mirror the Ghillie values structure but align security context, probes, and
config/secret patterns with the Nile Valley `example-app`:

- `image.repository`, `image.tag`, `image.pullPolicy`
- `replicaCount`
- `service.port` (default 8080), `service.type` (ClusterIP)
- `ingress.enabled`, `ingress.className`, `ingress.hosts`, `ingress.tls`
- `config` (map of non-secret env vars, rendered as ConfigMap)
- `existingSecretName` (pre-created Secret for DATABASE_URL, VALKEY_URL)
- `externalSecret.*` (ESO configuration for GitOps)
- `resources`, `securityContext`, `podSecurityContext` (hardened defaults
  matching `example-app`)
- `container.livenessProbe`, `container.readinessProbe`,
  `container.startupProbe` (defaulting to `/health/live` and `/health/ready` on
  port `http`)
- `serviceAccount.create`, `serviceAccount.name`,
  `serviceAccount.annotations`
- `pdb.enabled`, `pdb.minAvailable`
- `nameOverride`, `fullnameOverride`

**C.3: Templates.**

Create templates in `charts/corbusier/templates/`:

- `_helpers.tpl` — standard name, fullname, labels, selector labels
  helpers.
- `deployment.yaml` — Deployment with security context, topology spread,
  probes, config/secret env injection matching `example-app` patterns.
- `service.yaml` — ClusterIP Service.
- `ingress.yaml` — conditional Ingress (hostless for local k3d, explicit
  host for GitOps).
- `configmap.yaml` — ConfigMap from `.Values.config`.
- `serviceaccount.yaml` — conditional ServiceAccount.
- `externalsecret.yaml` — conditional ExternalSecret for ESO.
- `pdb.yaml` — conditional PodDisruptionBudget.
- `NOTES.txt` — post-install message.

**C.4: Values schema (`charts/corbusier/values.schema.json`).**

JSON Schema validating the values interface, matching the pattern in Ghillie's
chart.

**Validation for Stage C:**

```bash
helm lint charts/corbusier/
helm template corbusier charts/corbusier/ \
  --set image.tag=local \
  --set existingSecretName=corbusier
# Inspect output for correctness.
```

### Stage D: Local k3d lifecycle script

Create `scripts/local_k8s.py` and `scripts/local_k8s/` package, following the
Ghillie implementation structure and Corbusier's scripting standards.

**D.1: Configuration (`scripts/local_k8s/config.py`).**

Frozen dataclass with defaults for Corbusier:

- `cluster_name`: `"corbusier-local"`
- `namespace`: `"corbusier"`
- `app_name`: `"corbusier"`
- `chart_path`: `Path("charts/corbusier")`
- `image_repo`: `"corbusier"`
- `image_tag`: `"local"`
- `cnpg_release`, `cnpg_namespace`: same as Ghillie
- `valkey_release`, `valkey_namespace`: same as Ghillie
- `values_file`: `Path("charts/corbusier/values.yaml")` (or a dedicated
  `values.local.yaml`)

**D.2: Package modules.**

Port from Ghillie's `scripts/local_k8s/` package, adapting for Corbusier's
naming:

- `__init__.py`
- `config.py` (D.1 above)
- `validation.py` — `require_exe`, `pick_free_loopback_port`,
  `LocalK8sError`, `PortMismatchError`
- `k3d.py` — cluster lifecycle (create, delete, exists, kubeconfig,
  import image, get ingress port)
- `k8s.py` — namespace creation
- `cnpg.py` — CNPG operator install, cluster creation, readiness wait,
  URI extraction
- `valkey.py` — Valkey operator install, instance creation, readiness
  wait, URI extraction
- `deployment.py` — Docker build, app secret creation, chart install,
  status, logs
- `orchestration.py` — high-level `setup_environment`,
  `teardown_environment`, `show_environment_status`, `stream_environment_logs`

These modules use `plumbum` for subprocess execution per the scripting
standards (Ghillie uses raw `subprocess`; Corbusier should use `plumbum`).

**D.3: CLI entry point (`scripts/local_k8s.py`).**

Cyclopts app with subcommands `up`, `down`, `status`, `logs`, mirroring
Ghillie's CLI. Environment variables prefixed `CORBUSIER_K3D_`.

```python
#!/usr/bin/env -S uv run python
# /// script
# requires-python = ">=3.13"
# dependencies = ["cyclopts>=2.9", "plumbum", "cmd-mox"]
# ///
```

**Validation for Stage D:**

```bash
uv run scripts/local_k8s.py up
# Expected: cluster created, CNPG ready, Valkey ready, chart deployed,
# preview URL printed.
curl http://127.0.0.1:<port>/health/live
# Expected: 200 OK, body "ok"
uv run scripts/local_k8s.py status
# Expected: pods shown as Running/Ready.
uv run scripts/local_k8s.py down
# Expected: cluster deleted.
```

### Stage E: Makefile targets, documentation, and validation

**E.1: Add Makefile targets.**

Add to the existing `Makefile`:

```makefile
local-k8s-up: ## Create local k3d preview environment
	uv run scripts/local_k8s.py up

local-k8s-down: ## Delete local k3d preview environment
	uv run scripts/local_k8s.py down

local-k8s-status: ## Show local preview environment status
	uv run scripts/local_k8s.py status

local-k8s-logs: ## Tail application logs from preview environment
	uv run scripts/local_k8s.py logs
```

**E.2: Create `values.local.yaml`.**

A local-development values override file at `charts/corbusier/` or at the
repository root:

```yaml
image:
  repository: corbusier
  tag: local
ingress:
  enabled: true
  hosts:
    - host: ""
      paths:
        - path: /
          pathType: Prefix
existingSecretName: corbusier
externalSecret:
  enabled: false
```

**E.3: Write `docs/local-k8s-preview-design.md`.**

A design document for Corbusier's local preview environment, modelled on
Ghillie's `docs/local-k8s-preview-design.md`, covering:

- Goals, non-goals, and constraints.
- Helm chart design (values interface, ingress behaviour, secrets).
- Container image design (build strategy, entrypoint, tagging).
- Local k3d lifecycle script design (CLI shape, cluster creation,
  platform dependencies, secrets, idempotence).
- GitOps alignment (FluxCD HelmRelease, Kustomize overlays, CI pipeline
  image tags).
- Sequence diagram of the local preview flow.

**E.4: Update `docs/roadmap.md`.**

Add a section for local k3d preview and Nile Valley integration work.

**Validation for Stage E:**

Run the full end-to-end flow:

```bash
make local-k8s-up
make local-k8s-status
curl http://127.0.0.1:<port>/health/live
make local-k8s-logs
make local-k8s-down
make markdownlint
```

Confirm all existing targets still pass:

```bash
make all
```

## Concrete steps

Steps are listed per stage. Working directory is the Corbusier repository root
(`/data/leynos/Projects/corbusier.worktrees/nile-valley-integration` or the
main worktree).

### Stage B

1. Add `actix-web` dependency to `Cargo.toml`. (Escalation point: new
   dependency requires approval.)
2. Create `src/health/mod.rs` with `HealthStatus` enum, `HealthCheck`
   trait (port), and `SimpleHealthCheck` implementation.
3. Create `src/health/actix_adapter.rs` with HTTP handler functions and
   `health_routes` configurator.
4. Register `pub mod health;` in `src/lib.rs`.
5. Replace the stub `src/main.rs` with an `#[actix_web::main]` entry
   point that wires the port to the adapter.
6. Run `make test && make lint && make check-fmt` — expect all green.
7. Create `Dockerfile` and `.dockerignore` at repository root.
8. Run `docker build -t corbusier:local .` — expect successful build.
9. Run container and verify health endpoints respond.

### Stage C

1. Create `charts/corbusier/Chart.yaml`.
2. Create `charts/corbusier/values.yaml`.
3. Create all templates in `charts/corbusier/templates/`.
4. Create `charts/corbusier/values.schema.json`.
5. Run `helm lint charts/corbusier/` — expect no errors.
6. Run `helm template` and inspect output.

### Stage D

1. Create `scripts/local_k8s/` package with all modules.
2. Create `scripts/local_k8s.py` CLI entry point.
3. Run `uv run scripts/local_k8s.py up` — expect cluster, infra, and
   app deployment.
4. Verify health endpoint via `curl`.
5. Run `uv run scripts/local_k8s.py down` — expect clean teardown.

### Stage E

1. Add Makefile targets.
2. Create `values.local.yaml`.
3. Write `docs/local-k8s-preview-design.md`.
4. Update `docs/roadmap.md`.
5. Run `make markdownlint`.
6. Run full end-to-end flow via Make targets.

## Validation and acceptance

Quality criteria (what "done" means):

- `make local-k8s-up` creates a working k3d cluster with Corbusier
  running, Postgres provisioned by CNPG, and Valkey provisioned by the Valkey
  operator.
- `curl http://127.0.0.1:<port>/health/live` returns HTTP 200.
- `make local-k8s-status` shows all pods in Running/Ready state.
- `make local-k8s-down` cleanly deletes the cluster.
- `helm lint charts/corbusier/` passes.
- `helm template` produces valid Kubernetes manifests.
- `make all` (existing Rust checks) continues to pass.
- `make markdownlint` passes on all new and modified Markdown files.
- The Helm chart can be deployed via a FluxCD HelmRelease in the Nile
  Valley platform without modification (verified by `helm template` with
  production-like values).

Quality method (how we check):

- Run the concrete steps above in sequence.
- Inspect `helm template` output for correct labels, security contexts,
  probes, and secret references.
- Verify `docker build` completes and the image runs.

## Idempotence and recovery

- `make local-k8s-up` is safe to run when the cluster already exists; it
  reuses the existing cluster and re-deploys.
- `make local-k8s-down` is safe to run when no cluster exists; it exits
  cleanly.
- `docker build` is idempotent via layer caching.
- `helm upgrade --install` is idempotent.
- If any step fails partway, running `make local-k8s-down` followed by
  `make local-k8s-up` provides a clean restart.

## Artifacts and notes

### Nile Valley `example-app` chart conventions to adopt

The `example-app` chart uses these patterns that Corbusier should match:

- Security context: `runAsNonRoot: true`, `runAsUser: 10001`,
  `readOnlyRootFilesystem: true`, `allowPrivilegeEscalation: false`,
  `capabilities.drop: [ALL]`, `seccompProfile: RuntimeDefault`.
- Health probes: `/health/live` (liveness), `/health/ready` (readiness
  and startup), port named `http`.
- Config: non-secret values in a ConfigMap, secrets via
  `existingSecretName` with `secretKeyRef`.
- Labels: standard `app.kubernetes.io/*` labels with `component` and
  `part-of`.
- Topology spread: zone and hostname constraints.

### Ghillie modules to port

The following Ghillie `scripts/local_k8s/` modules are ported with adaptations
for Corbusier naming and `plumbum` subprocess execution:

- `config.py` — rename defaults (cluster name, namespace, app name,
  chart path).
- `validation.py` — reusable as-is with minor naming changes.
- `k3d.py` — reusable with `plumbum` adaptation.
- `k8s.py` — reusable as-is.
- `cnpg.py` — reusable with naming changes (pg cluster name).
- `valkey.py` — reusable with naming changes.
- `deployment.py` — adapt for Corbusier image name and chart path.
- `orchestration.py` — adapt for Corbusier naming and flow.

## Interfaces and dependencies

### Rust (new, Stage B)

In `Cargo.toml`, add:

```toml
actix-web = "4"
```

Tokio is already present with `rt-multi-thread` and `macros` features.

In `src/health/mod.rs` (domain port):

```rust
/// Health status reported by the application.
pub enum HealthStatus { Healthy, Degraded, Unhealthy }

/// Port for health observation.
pub trait HealthCheck: Send + Sync {
    fn liveness(&self) -> HealthStatus;
    fn readiness(&self) -> HealthStatus;
}
```

In `src/health/actix_adapter.rs` (driving adapter):

```rust
pub fn health_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/health/live", web::get().to(liveness))
       .route("/health/ready", web::get().to(readiness));
}
```

In `src/main.rs` (composition root):

```rust
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let health: Arc<dyn HealthCheck> = Arc::new(SimpleHealthCheck);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::from(health.clone()))
            .configure(health_routes)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
```

### Helm chart (new, Stage C)

The chart exposes the values interface documented in Stage C.2 above. Templates
follow the Nile Valley `example-app` label and security conventions.

### Python scripts (new, Stage D)

The `scripts/local_k8s` package exposes these public functions from
`orchestration.py`:

```python
def setup_environment(
    cluster_name: str,
    namespace: str,
    ingress_port: int | None,
    *,
    skip_build: bool,
) -> int: ...

def teardown_environment(cluster_name: str) -> int: ...

def show_environment_status(
    cluster_name: str, namespace: str
) -> int: ...

def stream_environment_logs(
    cluster_name: str, namespace: str, *, follow: bool
) -> int: ...
```
