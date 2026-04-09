.PHONY: help all clean test typecheck build release lint fmt check-fmt markdownlint nixie local-k8s-up local-k8s-down local-k8s-status local-k8s-logs frontend-install frontend-dev frontend-lint frontend-typecheck frontend-test frontend-test-a11y frontend-localizability frontend-semantic frontend-e2e frontend-audit


TARGET ?= corbusier

CARGO ?= cargo
BUN ?= bun
BUILD_JOBS ?=
RUST_FLAGS ?= -D warnings
RUSTDOC_FLAGS ?=
CARGO_FLAGS ?= --all-targets --all-features
CLIPPY_FLAGS ?= $(CARGO_FLAGS) -- $(RUST_FLAGS)
TEST_FLAGS ?= $(CARGO_FLAGS)
MDLINT ?= markdownlint-cli2
NIXIE ?= nixie
FRONTEND_DIR ?= frontend-pwa

build: target/debug/$(TARGET) ## Build debug binary
release: target/release/$(TARGET) ## Build release binary

all: check-fmt lint test ## Perform a comprehensive check of code

clean: ## Remove build artifacts
	$(CARGO) clean

test: ## Run tests with warnings treated as errors
	RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) nextest run $(TEST_FLAGS) $(BUILD_JOBS)

typecheck: ## Run cargo type checks across the workspace
	RUSTFLAGS="$(RUST_FLAGS)" $(CARGO) check $(CARGO_FLAGS) $(BUILD_JOBS)

target/%/$(TARGET): ## Build binary in debug or release mode
	$(CARGO) build $(BUILD_JOBS) $(if $(findstring release,$(@)),--release) --bin $(TARGET)

lint: ## Run Clippy with warnings denied
	RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" $(CARGO) doc --no-deps
	$(CARGO) clippy $(CLIPPY_FLAGS)
	RUSTFLAGS="$(RUST_FLAGS)" whitaker --all -- $(CARGO_FLAGS)

fmt: ## Format Rust and Markdown sources
	$(CARGO) fmt --all
	if command -v fd >/dev/null 2>&1; then \
		fd --print0 --type f --extension md --extension markdown --extension mdx . | \
			xargs -0 mdtablefix --wrap --renumber --breaks --ellipsis --fences --in-place; \
		fd --print0 --type f --extension md --extension markdown --extension mdx . | \
			xargs -0 markdownlint --fix; \
	else \
		find . \
			\( -path '*/node_modules/*' -o -path '*/.venv/*' -o -path '*/target/*' \) -prune -o \
			-type f \( -name '*.md' -o -name '*.markdown' -o -name '*.mdx' \) -print0 | \
			xargs -0 mdtablefix --wrap --renumber --breaks --ellipsis --fences --in-place; \
		find . \
			\( -path '*/node_modules/*' -o -path '*/.venv/*' -o -path '*/target/*' \) -prune -o \
			-type f \( -name '*.md' -o -name '*.markdown' -o -name '*.mdx' \) -print0 | \
			xargs -0 markdownlint --fix; \
	fi

check-fmt: ## Verify formatting
	$(CARGO) fmt --all -- --check

markdownlint: ## Lint Markdown files
	$(MDLINT) '**/*.md'

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
	cd $(FRONTEND_DIR) && $(BUN) install
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

frontend-audit: ## Audit frontend dependencies for known vulnerabilities
	cd $(FRONTEND_DIR) && $(BUN) run audit

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | \
	awk 'BEGIN {FS=":"; printf "Available targets:\n"} {printf "  %-20s %s\n", $$1, $$2}'
