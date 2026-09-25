.PHONY: help all clean test typecheck build release lint fmt check-fmt markdownlint spelling test-workflow-contracts nixie local-k8s-up local-k8s-down local-k8s-status local-k8s-logs frontend-install frontend-dev frontend-lint frontend-typecheck frontend-test frontend-test-a11y frontend-localizability frontend-semantic frontend-e2e audit audit-node rust-audit

TARGET ?= corbusier

CARGO ?= $(shell command -v cargo 2>/dev/null || printf '%s/.cargo/bin/cargo' "$$HOME")
WHITAKER ?= $(shell command -v whitaker 2>/dev/null || \
	if [ -x "$$HOME/.local/share/whitaker/whitaker" ]; then \
		printf '%s/.local/share/whitaker/whitaker' "$$HOME"; \
	elif [ -x "$$HOME/.local/bin/whitaker" ]; then \
		printf '%s/.local/bin/whitaker' "$$HOME"; \
	else \
		printf '%s/.cargo/bin/whitaker' "$$HOME"; \
	fi)
BUN ?= bun
BUILD_JOBS ?=
RUST_FLAGS ?= -D warnings
RUSTDOC_FLAGS ?=
CARGO_FLAGS ?= --all-targets --all-features
CLIPPY_FLAGS ?= $(CARGO_FLAGS) -- $(RUST_FLAGS)
TEST_FLAGS ?= $(CARGO_FLAGS)
MDLINT ?= $(shell if [ -n "$$HOME" ] && [ -x "$$HOME/.bun/bin/markdownlint-cli2" ]; then \
		printf '%s/.bun/bin/markdownlint-cli2' "$$HOME"; \
	elif command -v markdownlint-cli2 >/dev/null 2>&1; then \
		command -v markdownlint-cli2; \
	else \
		printf 'markdownlint-cli2'; \
	fi)
NIXIE ?= nixie
TYPOS_CONFIG_BUILDER_VERSION ?= v0.1.1
TYPOS_CONFIG_BUILDER := uv tool run --python 3.14 --from \
	"git+https://github.com/leynos/typos-config-builder.git@$(TYPOS_CONFIG_BUILDER_VERSION)" \
	typos-config-builder
# The workflow contracts are pytest modules with one YAML dependency. Both are
# pinned, and bytecode is not written: a mutation run that edits a module can
# otherwise execute a stale `.pyc` whose mtime and size match the edit.
WORKFLOW_PYTEST ?= PYTHONDONTWRITEBYTECODE=1 uv run --no-project --python 3.14 \
	--with pytest==9.0.2 --with pyyaml==6.0.3 python -m pytest
FRONTEND_DIR ?= frontend-pwa
FRONTEND_INSTALL_FLAGS ?=

build: target/debug/$(TARGET) ## Build debug binary
release: target/release/$(TARGET) ## Build release binary

all: check-fmt lint test ## Perform a comprehensive check of code

clean: ## Remove build artifacts
	$(CARGO) clean
	rm -f .typos-oxendict-base.json .typos-oxendict-base.toml

test: ## Run tests with warnings treated as errors
	RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) nextest run $(TEST_FLAGS) $(BUILD_JOBS)

typecheck: ## Run cargo type checks across the workspace
	RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) check $(CARGO_FLAGS) $(BUILD_JOBS)

target/%/$(TARGET): ## Build binary in debug or release mode
	$(CARGO) build $(BUILD_JOBS) $(if $(findstring release,$(@)),--release) --bin $(TARGET)

# `test-workflow-contracts` is a prerequisite here as well as a CI step of its
# own, so deleting the step does not stop the contracts running.
lint: test-workflow-contracts ## Run Clippy and the Whitaker Dylint suite with warnings denied
	RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" $(CARGO) doc --no-deps
	$(CARGO) clippy $(CLIPPY_FLAGS)
	PATH="$(dir $(CARGO)):$(dir $(WHITAKER)):$$PATH" RUSTFLAGS="$(RUST_FLAGS)" $(WHITAKER) --all -- $(CARGO_FLAGS)
	+$(MAKE) spelling

fmt: ## Format Rust and Markdown sources
	$(CARGO) fmt --all
	mdformat-all

check-fmt: ## Verify formatting
	$(CARGO) fmt --all -- --check

markdownlint: ## Lint Markdown files
	$(MDLINT) '**/*.md'
	+$(MAKE) spelling

spelling: ## Enforce en-GB-oxendict spelling in Markdown prose
	$(TYPOS_CONFIG_BUILDER) gate --repository .

test-workflow-contracts: ## Assert what the workflow files must say
	$(WORKFLOW_PYTEST) --doctest-modules tests/workflow_contracts -q

nixie: ## Validate Mermaid diagrams
	$(NIXIE) --no-sandbox

local-k8s-up: ## Create local k3d preview environment
	uv run scripts/local_k8s.py up

local-k8s-down: ## Delete local k3d preview environment
	uv run scripts/local_k8s.py down

local-k8s-status: ## Show local preview environment status
	uv run scripts/local_k8s.py status

local-k8s-logs: ## Tail application logs from preview environment
	uv run scripts/local_k8s.py logs

frontend-install: ## Install frontend workspace dependencies and browser tooling
	cd $(FRONTEND_DIR) && $(BUN) install $(FRONTEND_INSTALL_FLAGS)
	cd $(FRONTEND_DIR) && $(BUN) x playwright install chromium

frontend-dev: ## Run the frontend development server
	cd $(FRONTEND_DIR) && $(BUN) run dev --host 127.0.0.1 --port 4173

frontend-lint: ## Lint the frontend workspace
	cd $(FRONTEND_DIR) && $(BUN) run lint

frontend-typecheck: ## Type-check the frontend workspace
	cd $(FRONTEND_DIR) && $(BUN) run typecheck

frontend-test: ## Run frontend unit and component tests
	cd $(FRONTEND_DIR) && $(BUN) run test

frontend-test-a11y: ## Run frontend accessibility-focused component tests
	cd $(FRONTEND_DIR) && $(BUN) run test:a11y

frontend-localizability: ## Run frontend heuristic localizability checks
	cd $(FRONTEND_DIR) && $(BUN) run localizability:lint

frontend-semantic: ## Run semantic frontend linting and styling checks
	cd $(FRONTEND_DIR) && $(BUN) run semantic:lint

frontend-e2e: ## Run frontend browser-path tests
	cd $(FRONTEND_DIR) && $(BUN) run e2e

audit: audit-node rust-audit ## Audit frontend and Rust dependencies for known vulnerabilities

audit-node: ## Audit frontend dependencies for known vulnerabilities
	cd $(FRONTEND_DIR) && $(BUN) run audit

rust-audit: ## Audit every Rust manifest for known vulnerabilities
	find . \
		\( -path '*/target/*' -o -path '*/node_modules/*' -o -path '*/.venv/*' \) -prune -o \
		-name Cargo.toml -exec sh -c 'set -e; for manifest do \
			manifest_dir=$$(dirname "$$manifest"); \
			printf "Auditing Rust manifest %s\n" "$$manifest"; \
			(cd "$$manifest_dir" && $(CARGO) audit); \
		done' sh {} +

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | \
	awk 'BEGIN {FS=":"; printf "Available targets:\n"} {printf "  %-20s %s\n", $$1, $$2}'
