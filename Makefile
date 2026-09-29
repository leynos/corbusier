.PHONY: help all clean test typecheck build release lint fmt check-fmt markdownlint spelling test-workflow-contracts audit-exceptions audit-exceptions-test audit-commands-test nixie local-k8s-up local-k8s-down local-k8s-status local-k8s-logs frontend-install frontend-dev frontend-lint frontend-typecheck frontend-docs-check frontend-test frontend-test-a11y frontend-localizability frontend-semantic frontend-e2e audit audit-node rust-audit

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
# `make fmt` and `make check-fmt` call mdtablefix directly. `--git` selects the
# Markdown files Git tracks and `--include-untracked` adds the untracked files
# Git does not ignore, so a new document is formatted before it is staged.
# Both modes need mdtablefix 0.6.0 or later; CI pins the version at the
# install-mdtablefix step.
MDTABLEFIX ?= mdtablefix
MDTABLEFIX_SELECT = --git --include-untracked
MDTABLEFIX_RULES = --wrap --renumber --breaks --ellipsis --fences
BUN ?= bun
BUILD_JOBS ?=
RUST_FLAGS ?= -D warnings
RUSTDOC_FLAGS ?=
CARGO_FLAGS ?= --all-targets --all-features
CLIPPY_FLAGS ?= $(CARGO_FLAGS) -- $(RUST_FLAGS)
TEST_FLAGS ?= $(CARGO_FLAGS)
MDLINT ?= $(shell command -v markdownlint-cli2 2>/dev/null || printf '%s' "$$HOME/.bun/bin/markdownlint-cli2")
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
# The validator for dated Rust audit exceptions. A variable so the command
# tests can stand in for it and prove `rust-audit` runs it first.
AUDIT_EXCEPTIONS ?= uv run scripts/rust_audit_exceptions.py
# The audit-exception rule's tests need pytest alone.
AUDIT_PYTEST ?= PYTHONDONTWRITEBYTECODE=1 uv run --no-project --python 3.14 \
	--with pytest==9.0.2 python -m pytest
FRONTEND_DIR ?= frontend-pwa
FRONTEND_INSTALL_FLAGS ?=

UV ?= uv
UV_ENV = UV_CACHE_DIR=.uv-cache UV_TOOL_DIR=.uv-tools
# The CV-005 CodeScene contracts live in shared-actions and run from a full
# commit, so a fix is a pin bump. `.github/cv005.toml` holds this repository's
# only parameters.
CV005_CONTRACTS_REF ?= a38feb9be25755c30eca5bda96bd3786a5b89c6b
CV005_CONTRACTS = $(UV_ENV) $(UV) tool run --python 3.13 \
	--from 'git+https://github.com/leynos/shared-actions@$(CV005_CONTRACTS_REF)\#subdirectory=packages/cv005-contracts' \
	cv005-contracts

build: target/debug/$(TARGET) ## Build debug binary
release: target/release/$(TARGET) ## Build release binary

all: check-fmt lint test test-workflow-contracts ## Perform a comprehensive check of code

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
lint: test-workflow-contracts audit-commands-test ## Run Clippy and the Whitaker Dylint suite with warnings denied
	RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" $(CARGO) doc --no-deps
	$(CARGO) clippy $(CLIPPY_FLAGS)
	PATH="$(dir $(CARGO)):$(dir $(WHITAKER)):$$PATH" RUSTFLAGS="$(RUST_FLAGS)" $(WHITAKER) --all -- $(CARGO_FLAGS)
	+$(MAKE) spelling

fmt: ## Format Rust and Markdown sources
	$(CARGO) fmt --all
	$(MDTABLEFIX) --in-place $(MDTABLEFIX_SELECT) $(MDTABLEFIX_RULES)
	@unset FORCE_COLOR; $(MDLINT) --fix "**/*.md"

check-fmt: ## Verify formatting
	$(CARGO) fmt --all -- --check
	$(MDTABLEFIX) --check $(MDTABLEFIX_SELECT) $(MDTABLEFIX_RULES)

markdownlint: ## Lint Markdown files
	$(MDLINT) '**/*.md'
	+$(MAKE) spelling

spelling: ## Enforce en-GB-oxendict spelling in Markdown prose
	$(TYPOS_CONFIG_BUILDER) gate --repository .

test-workflow-contracts: ## Assert what the workflow files must say
	$(CV005_CONTRACTS) check --repository .
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

frontend-docs-check: ## Run the zero-tolerance TypeDoc documentation gate over frontend-pwa/src
	cd $(FRONTEND_DIR) && $(BUN) run docs:check

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

# Both halves always run and both report. As Make prerequisites the first
# failure stopped the target, so 24 frontend advisories masked
# RUSTSEC-2026-0258 for weeks. One failure must not hide another.
audit: ## Audit frontend and Rust dependencies, reporting both
	MAKE="$(MAKE)" uv run scripts/run_audits.py

audit-node: ## Audit frontend dependencies for known vulnerabilities
	cd $(FRONTEND_DIR) && $(BUN) run audit

audit-exceptions: audit-exceptions-test ## Refuse an ignored advisory that is undated or expired
	$(AUDIT_EXCEPTIONS)

audit-exceptions-test: ## Drive the exception rule and the audit runner over constructed cases
	@PYTHONPATH=scripts $(AUDIT_PYTEST) --doctest-modules \
		scripts/tests/test_rust_audit_exceptions.py scripts/tests/test_run_audits.py \
		scripts/audit_exception_blocks.py scripts/rust_audit_exceptions.py \
		scripts/run_audits.py

# The command tests run `make audit` in a child process, so they must never be
# reachable from `audit` itself: a stand-in `make` that is bypassed would
# recurse. `lint` reaches them instead.
audit-commands-test: ## Run the audit commands end to end with a stand-in make
	@PYTHONPATH=scripts $(AUDIT_PYTEST) scripts/tests/test_audit_commands.py

rust-audit: audit-exceptions ## Audit every Rust manifest for known vulnerabilities
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
