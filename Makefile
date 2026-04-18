# RFin Development Makefile
# -------------------------
# A developer-friendly entry point for building, testing, and linting the RFin project.

# --- Configuration & Macros ---

.DEFAULT_GOAL := help
SHELL := /bin/bash

# OS detection: identify Windows via MSYS / Git Bash / Cygwin uname output
UNAME_S := $(shell uname -s 2>/dev/null || echo unknown)
IS_WINDOWS := $(strip $(if $(or $(findstring MINGW,$(UNAME_S)),$(findstring MSYS,$(UNAME_S)),$(findstring CYGWIN,$(UNAME_S))),true,))

# Detect Python environment (Scripts/ on Windows, bin/ on Unix)
VENV := .venv
VENV_PATH := $(CURDIR)/$(VENV)
ifdef IS_WINDOWS
  VENV_BIN_DIR := Scripts
else
  VENV_BIN_DIR := bin
endif
PYTHON_ACTIVATE := [ -f "$(VENV_PATH)/$(VENV_BIN_DIR)/activate" ] && . "$(VENV_PATH)/$(VENV_BIN_DIR)/activate"
UV := $(shell command -v uv 2> /dev/null)

# Macro to run python commands consistently
# Use: $(call py_run,ruff format .)
define py_run
if [ -d "$(VENV_PATH)" ]; then \
	$(PYTHON_ACTIVATE) && $(1); \
elif [ -n "$(UV)" ]; then \
	uv run $(1); \
else \
	$(1); \
fi
endef

define py_run_no_sync
if [ -d "$(VENV_PATH)" ]; then \
	$(PYTHON_ACTIVATE) && $(1); \
elif [ -n "$(UV)" ]; then \
	uv run --no-sync $(1); \
else \
	$(1); \
fi
endef

# --- Help ---

.PHONY: help
help: ## Display this help message

	@printf "\n███████╗██╗███╗░░██╗░██████╗████████╗░█████╗░░█████╗░██╗░░██╗"
	@printf "\n██╔════╝██║████╗░██║██╔════╝╚══██╔══╝██╔══██╗██╔══██╗██║░██╔╝"
	@printf "\n█████╗░░██║██╔██╗██║╚█████╗░░░░██║░░░███████║██║░░╚═╝█████═╝░"
	@printf "\n██╔══╝░░██║██║╚████║░╚═══██╗░░░██║░░░██╔══██║██║░░██╗██╔═██╗░"
	@printf "\n██║░░░░░██║██║░╚███║██████╔╝░░░██║░░░██║░░██║╚█████╔╝██║░╚██╗"
	@printf "\n╚═╝░░░░░╚═╝╚═╝░░╚══╝╚═════╝░░░░╚═╝░░░╚═╝░░╚═╝░╚════╝░╚═╝░░╚═╝"
	@printf "\n\nRFin Development Makefile\n"
	@printf "Usage: make [target]\n\n"
	@printf "Main Targets:\n"
	@printf "  \033[36mbuild\033[0m               Build all core Rust crates\n"
	@printf "  \033[36mtest\033[0m                Run all tests across the project\n"
	@printf "  \033[36mfmt\033[0m                 Format all codebases\n"
	@printf "  \033[36mlint\033[0m                Lint all code (fast: core Rust crates only)\n"
	@printf "  \033[36mlint-full\033[0m           Lint all code including bindings + all features (slow)\n"
	@printf "  \033[36mci-test\033[0m             Run all checks exactly as CI does (wasm-build + pre-commit + test)\n\n"
	@printf "Component Specifics:\n"
	@printf "  \033[36mtest-rust\033[0m           Run Rust tests (cargo-nextest)\n"
	@printf "  \033[36mtest-python\033[0m         Run Python tests\n"
	@printf "  \033[36mtest-python-cov\033[0m     Run Python tests with coverage report\n"
	@printf "  \033[36mexamples-python\033[0m     Run all Python examples (scripts & notebooks)\n"
	@printf "  \033[36mtest-wasm\033[0m           Run WASM package tests\n"
	@printf "  \033[36mtest-wasm-cov\033[0m       Run WASM binding tests with coverage report\n\n"
	@printf "Setup & Maintenance:\n"
	@printf "  \033[36msetup-python\033[0m        Initialize Python environment with uv\n"
	@printf "  \033[36mpython-dev\033[0m          Install Python deps and build bindings (release)\n"
	@printf "  \033[36mpython-dev-debug\033[0m    Install Python deps and build bindings (debug, fast compile)\n"
	@printf "  \033[36mclean\033[0m               Remove build artifacts and virtualenvs\n\n"
	@printf "Documentation:\n"
	@printf "  \033[36mdoc\033[0m                 Generate Rust documentation\n"
	@printf "  \033[36mbook-serve\033[0m          Serve mdBook with live reload\n"
	@printf "  \033[36mpython-docs\033[0m         Build Python API docs (mkdocs)\n"
	@printf "  \033[36mpython-docs-serve\033[0m   Serve Python API docs with live reload\n"
	@printf "  \033[36mdocs-all\033[0m            Build complete documentation site\n\n"
	@printf "Development & Tooling:\n"
	@printf "  \033[36mgenerate-bindings\033[0m   Export TypeScript types from Rust\n"
	@printf "  \033[36mexamples-python-scripts\033[0m   Run Python example scripts\n"
	@printf "  \033[36mexamples-python-notebooks\033[0m Run Python example notebooks\n"
	@printf "  \033[36mcheck-env\033[0m           Verify development environment\n"
	@printf "  \033[36mupdate\033[0m              Update all dependencies (Rust, Python, JS)\n"
	@printf "  \033[36maudit\033[0m               Run security audits on all components\n\n"
	@printf "Packaging & Distribution:\n"
	@printf "  \033[36mwheel-local\033[0m         Build wheel for current platform + Python\n"
	@printf "  \033[36mwheel-docker\033[0m        Build manylinux wheel via Docker\n"
	@printf "  \033[36mwheel-all\033[0m           Build wheels for all local Python versions\n"
	@printf "  \033[36mwasm-pkg\033[0m            Build WASM package (web + node targets)\n"
	@printf "  \033[36mwasm-publish-dry\033[0m    Dry-run npm publish\n\n"
	@printf "Analysis & Coverage:\n"
	@printf "  \033[36mcoverage\033[0m            Run coverage for all components\n"
	@printf "  \033[36mcoverage-rust\033[0m       Run Rust code coverage\n"
	@printf "  \033[36mcoverage-rust-gate\033[0m  Enforce Rust workspace and touched-file coverage gates\n"
	@printf "  \033[36mcoverage-python\033[0m     Run Python code coverage\n"
	@printf "  \033[36mlist\033[0m                Generate API parity report\n"
	@printf "  \033[36msize-all\033[0m            Analyze binary sizes\n\n"
	@printf "Run 'make list' for API parity reports or 'make size-all' for binary analysis.\n"

# --- Primary Targets ---

.PHONY: all
all: build test lint ## Build, test, and lint everything

.PHONY: build
build: ## Build all crates (excluding python/wasm)
	CARGO_INCREMENTAL=1 cargo build --workspace --exclude finstack-py --exclude finstack-wasm

.PHONY: build-prod
build-prod: ## Build all crates optimized without debug info
	CARGO_INCREMENTAL=1 RUSTFLAGS="-C debuginfo=0" cargo build --workspace --exclude finstack-py --exclude finstack-wasm --release

.PHONY: test
test: test-rust test-python test-wasm ## Run all tests across all components

.PHONY: fmt
fmt: fmt-rust fmt-python fmt-wasm ## Format all code (Rust, Python, WASM)

.PHONY: lint
lint: lint-rust lint-python lint-wasm ## Check all code for linting issues (fast: core crates only)

.PHONY: lint-full
lint-full: lint-rust-full lint-python lint-wasm ## Check all code including bindings + all features (slow)

# --- Component: Rust ---

.PHONY: test-rust
test-rust: install-nextest
	CARGO_INCREMENTAL=1 cargo nextest run --workspace --exclude finstack-py --features mc --lib --test '*' --no-fail-fast

.PHONY: test-rust-slow
test-rust-slow: install-nextest
	CARGO_INCREMENTAL=1 cargo nextest run --workspace --exclude finstack-py --features mc --lib --test '*' --run-ignored all

.PHONY: test-rust-doc
test-rust-doc: check-no-doctest-ignore
	CARGO_INCREMENTAL=1 cargo test --workspace --exclude finstack-py --doc --features mc

.PHONY: fmt-rust
fmt-rust: ## Format and fix Rust code
	cargo fmt --all
	CARGO_INCREMENTAL=1 cargo clippy --all-targets --features mc --fix --allow-dirty -- -D warnings

.PHONY: lint-rust
lint-rust: ## Lint core Rust crates (fast: excludes bindings)
	cargo fmt --all -- --check
	CARGO_INCREMENTAL=1 cargo clippy --all-targets --features mc

.PHONY: lint-rust-full
lint-rust-full: ## Lint all Rust crates including bindings with all features (slow)
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets --all-features

.PHONY: check-no-doctest-ignore
check-no-doctest-ignore:
	@set -e; \
	if rg -n '^[[:space:]]*```[^\n]*\bignore\b' --glob '**/*.rs' finstack finstack-wasm finstack-py/src ; then \
		printf "ERROR: Found doctest code fences using 'ignore'.\n"; \
		printf "Use 'rust,no_run' for compile-only examples, 'rust' for runnable examples, or 'text' for non-Rust snippets.\n"; \
		exit 1; \
	fi

.PHONY: examples
examples: ## Run all Rust examples with nice categorization
	@./scripts/run-examples.sh

# --- Component: Python ---

.PHONY: setup-python
setup-python: ## Initialize Python environment
	@printf "Setting up Python development environment...\n"
	uv venv
	@printf "Virtual environment created. Now run: source .venv/$(VENV_BIN_DIR)/activate && make python-dev\n"

.PHONY: python-dev
python-dev: ## Install dependencies and build bindings (release mode)
	@if [ ! -d "$(VENV)" ]; then uv venv; fi
	@printf "Installing Python dependencies and building extension...\n"
	@$(call py_run,uv pip install maturin pytest pytest-benchmark ty ruff ipython jupyter)
	@cd finstack-py && $(call py_run,python -m maturin develop --profile $(or $(MATURIN_PROFILE),release))

.PHONY: python-dev-debug
python-dev-debug: ## Install dependencies and build bindings (debug mode, fast compile)
	@if [ ! -d "$(VENV)" ]; then uv venv; fi
	@printf "Installing Python dependencies and building extension (debug)...\n"
	@$(call py_run,uv pip install maturin pytest pytest-benchmark ty ruff ipython jupyter)
	@cd finstack-py && $(call py_run,python -m maturin develop)

.PHONY: test-python
test-python: ## Run Python tests
	@$(call py_run,pytest -v)

.PHONY: test-python-cov
test-python-cov: ## Run Python tests with coverage report
	@$(call py_run,pytest -v --cov=finstack-py/finstack --cov-report=term-missing --cov-report=html:target/python-cov)
	@printf "HTML report: target/python-cov/index.html\n"

.PHONY: fmt-python
fmt-python: ## Format and fix Python code
	uv run --no-sync ruff format .
	uv run --no-sync ruff check . --fix --unsafe-fixes

.PHONY: lint-python
lint-python: ## Lint Python code
	uv run --no-sync ruff format . --check
	uv run --no-sync ruff check . --no-fix

.PHONY: typecheck-python
typecheck-python:
	@$(call py_run_no_sync,ty check finstack-py/finstack)
	@$(call py_run_no_sync,pyright)

.PHONY: verifytypes-python
verifytypes-python:
	@$(call py_run_no_sync,pyright --verifytypes finstack --ignoreexternal)

.PHONY: examples-python examples-python-scripts examples-python-notebooks
examples-python: examples-python-scripts examples-python-notebooks ## Run all Python examples

examples-python-scripts: ## Run all Python example scripts
	@printf "Running Python example scripts...\n"
	@$(call py_run,python finstack-py/examples/scripts/run_all_examples.py)

examples-python-notebooks: ## Run all Python example notebooks
	@printf "Running Python example notebooks...\n"
	@$(call py_run,python finstack-py/examples/notebooks/run_all_notebooks.py)

# --- Component: WASM ---

.PHONY: wasm-build
wasm-build: ## Build WASM package
	cd finstack-wasm && wasm-pack build --target web

.PHONY: wasm-examples-dev
wasm-examples-dev: wasm-build
	cd finstack-wasm && npm run examples:install && npm run examples:dev

.PHONY: test-wasm
test-wasm:
	cd finstack-wasm && npm run test

WASM_COV_OUTPUT_DIR := target/wasm-cov
WASM_COV_PROFILE_DIR := $(abspath $(WASM_COV_OUTPUT_DIR))/profraw
WASM_COV_PROFILE_PATTERN := $(WASM_COV_PROFILE_DIR)/default_%m_%p.profraw
WASM_COV_IGNORE := '(tests?/|target/|\.cargo/)'
WASM_COV_BASE := LLVM_PROFILE_FILE="$(WASM_COV_PROFILE_PATTERN)" CARGO_INCREMENTAL=1 cargo llvm-cov -p finstack-wasm --lib --ignore-filename-regex $(WASM_COV_IGNORE)

.PHONY: test-wasm-cov
test-wasm-cov: ## Run WASM binding tests with coverage report
	@printf "Running WASM binding coverage...\n"
	@mkdir -p $(WASM_COV_PROFILE_DIR)
	$(WASM_COV_BASE) --html --output-dir $(WASM_COV_OUTPUT_DIR)
	@printf "HTML report: $(WASM_COV_OUTPUT_DIR)/html/index.html\n"

.PHONY: fmt-wasm
fmt-wasm: ## Format and fix WASM/TS code
	cd finstack-wasm && npm run format:fix
	cd finstack-wasm && npm run lint:fix || true

.PHONY: lint-wasm
lint-wasm: ## Lint WASM/TS code
	cd finstack-wasm && npm run format
	cd finstack-wasm && npm run lint

.PHONY: generate-bindings
generate-bindings: ## Export TypeScript types from Rust
	@printf "Generating TypeScript bindings...\n"
	cargo run -p finstack-wasm --bin ts_export --features ts_export

# --- Documentation ---

.PHONY: doc
doc: ## Generate and open rustdoc
	CARGO_INCREMENTAL=1 cargo doc --workspace --exclude finstack-py --exclude finstack-wasm --no-deps --all-features --open

.PHONY: python-docs
python-docs: ## Build Python API docs (mkdocs)
	$(call py_run,mkdocs build -f mkdocs.yml -d site/api/python)

.PHONY: python-docs-serve
python-docs-serve: ## Serve Python API docs with live reload
	$(call py_run,mkdocs serve -f mkdocs.yml)

.PHONY: docs-all
docs-all: doc book-build python-docs ## Build complete documentation site
	bash scripts/build-docs.sh

.PHONY: book-build
book-build: install-mdbook
	cd book && mdbook build

.PHONY: book-serve
book-serve: install-mdbook
	@printf "Documentation will be available at http://localhost:3000\n"
	cd book && mdbook serve --open

.PHONY: book-watch
book-watch: install-mdbook
	cd book && mdbook watch

.PHONY: book-clean
book-clean:
	rm -rf book/book

# --- Analysis & Quality ---

.PHONY: list
list: ## Generate API parity report
	@printf "Generating API parity report...\n"
	@$(call py_run,python scripts/audits/audit_rust_api.py)
	@$(call py_run,python scripts/audits/audit_python_api.py)
	@$(call py_run,python scripts/audits/audit_wasm_api.py)
	@$(call py_run,python scripts/audits/compare_apis.py)
	@printf "Done: PARITY_AUDIT.md\n"

COV_IGNORE := '(tests?/|target/|\.cargo/|.*finstack-py/.*|.*finstack-wasm/.*)'
COV_OUTPUT_DIR := target/llvm-cov
COV_JSON_SUMMARY := $(COV_OUTPUT_DIR)/rust-summary.json
COV_MIN_LINES := 75.0
COV_PROFILE_DIR := $(abspath $(COV_OUTPUT_DIR))/profraw
COV_PROFILE_PATTERN := $(COV_PROFILE_DIR)/default_%m_%p.profraw
COV_BASE := LLVM_PROFILE_FILE="$(COV_PROFILE_PATTERN)" CARGO_INCREMENTAL=1 cargo llvm-cov --workspace --exclude finstack-py --exclude finstack-wasm --features mc --ignore-filename-regex $(COV_IGNORE)

.PHONY: coverage coverage-rust coverage-rust-json coverage-rust-gate coverage-python coverage-html coverage-lcov
coverage: coverage-rust coverage-python ## Run all coverage reports

coverage-rust:
	@printf "Running Rust code coverage...\n"
	@mkdir -p $(COV_PROFILE_DIR)
	$(COV_BASE)

coverage-rust-json:
	@mkdir -p $(COV_OUTPUT_DIR)
	@mkdir -p $(COV_PROFILE_DIR)
	$(COV_BASE) --summary-only --json --output-path $(COV_JSON_SUMMARY)

coverage-rust-gate:
	@printf "Running Rust coverage gate...\n"
	@$(MAKE) coverage-rust-json
	@$(call py_run,python scripts/check_rust_coverage_gate.py --coverage-json $(COV_JSON_SUMMARY) --workspace-threshold $(COV_MIN_LINES) --touched-threshold $(COV_MIN_LINES))

coverage-python:
	@printf "Running Python code coverage...\n"
	@$(call py_run,pytest --cov=finstack-py/finstack --cov-report=html)

coverage-html: ## Generate Rust HTML coverage report (pass OPEN=1 to auto-open)
	@mkdir -p $(COV_PROFILE_DIR)
	$(COV_BASE) --html $(if $(OPEN),--open,)

coverage-lcov:
	@mkdir -p $(COV_PROFILE_DIR)
	$(COV_BASE) --lcov --output-path coverage.lcov

.PHONY: check-schemas
check-schemas: ## Verify JSON schemas match Rust types
	@$(call py_run,python scripts/sync_instrument_schema_overrides.py)
	cargo nextest run -p finstack-valuations schema_parity --no-fail-fast
	cargo test -p finstack-valuations test_instrument_schema_enum_parity --no-fail-fast
	cargo test -p finstack-valuations --test schema_audit --no-fail-fast

.PHONY: gen-schemas
gen-schemas: ## Regenerate typed JSON schemas from Rust types via schemars
	@$(call py_run,python scripts/sync_instrument_schema_overrides.py)
	cargo run -p finstack-valuations --bin gen_schemas

.PHONY: sync-versions
sync-versions: ## Sync sub-package versions from root Cargo.toml
	./scripts/sync-versions

.PHONY: sync-schemas
sync-schemas: ## Apply local schema override sync
	@$(call py_run,python scripts/sync_instrument_schema_overrides.py)

.PHONY: check-dups
check-dups:
	npx jscpd --pattern "**/src/**/*.rs" --ignore "**/target/**,**/node_modules/**,**/tests/**"

.PHONY: audit audit-rust audit-python
audit: audit-rust audit-python ## Run security audits on all components
audit-rust:
	@printf "Auditing Rust dependencies...\n"
	@command -v cargo-audit >/dev/null 2>&1 || { printf "cargo-audit not found. Install with: cargo install cargo-audit\n"; exit 1; }
	cargo audit
audit-python:
	@printf "Auditing Python dependencies...\n"
	@$(call py_run,bandit -r finstack-py -c pyproject.toml)
.PHONY: update update-rust update-python
update: update-rust update-python ## Update all dependencies
update-rust:
	@printf "Updating Rust dependencies...\n"
	cargo update
update-python:
	@printf "Updating Python dependencies...\n"
	uv lock --upgrade
.PHONY: check-env
check-env: ## Verify development environment
	@printf "Checking development environment...\n"
	@command -v cargo >/dev/null 2>&1 && printf "✅ Rust (cargo) is installed\n" || printf "❌ Rust (cargo) is missing\n"
	@command -v uv >/dev/null 2>&1 && printf "✅ uv (Python) is installed\n" || printf "❌ uv (Python) is missing\n"
	@command -v node >/dev/null 2>&1 && printf "✅ Node.js is installed\n" || printf "❌ Node.js is missing\n"
	@command -v npm >/dev/null 2>&1 && printf "✅ npm is installed\n" || printf "❌ npm is missing\n"
	@command -v wasm-pack >/dev/null 2>&1 && printf "✅ wasm-pack is installed\n" || printf "❌ wasm-pack is missing\n"
	@command -v mdbook >/dev/null 2>&1 && printf "✅ mdbook is installed\n" || printf "❌ mdbook is missing\n"
	@command -v cargo-nextest >/dev/null 2>&1 && printf "✅ cargo-nextest is installed\n" || printf "❌ cargo-nextest is missing\n"

# --- Benchmarking & Profiling ---

.PHONY: bench-perf
bench-perf:
	cargo bench --profile bench

.PHONY: bench-baseline
bench-baseline:
	cargo bench -- --save-baseline main

.PHONY: bench-flamegraph
bench-flamegraph:
	@command -v flamegraph >/dev/null 2>&1 || { printf "Installing flamegraph...\n"; cargo install flamegraph; }
	cargo flamegraph --bench mc_pricing --profile bench --features mc -- --bench

.PHONY: bench-compare
bench-compare:
	cargo bench -- --baseline main

# --- Binary Size Analysis ---

.PHONY: size-wasm size-py size-core size-all
size-wasm: install-twiggy
	cd finstack-wasm && wasm-pack build --target web --release
	@printf "\n=== WASM file sizes ===\n"
	@wasm_file=$$(ls -1 finstack-wasm/pkg/*.wasm 2>/dev/null | head -1); \
	if [ -n "$$wasm_file" ]; then \
		raw=$$(wc -c < "$$wasm_file" | tr -d ' '); \
		gz=$$(gzip -c "$$wasm_file" | wc -c | tr -d ' '); \
		printf "  Raw:    %s bytes (%s KiB)\n" "$$raw" "$$(( raw / 1024 ))"; \
		printf "  Gzipped: %s bytes (%s KiB)\n" "$$gz" "$$(( gz / 1024 ))"; \
		printf "\n=== Top functions by size ===\n"; \
		twiggy top -n 20 "$$wasm_file"; \
	else \
		printf "  No .wasm file found in finstack-wasm/pkg/\n"; \
	fi
size-py: install-bloat
	cd finstack-py && cargo build --release
	cargo bloat --release --crates -p finstack-py
size-core:
	@printf "finstack-core contribution in WASM binary:\n"
	@$(MAKE) size-wasm 2>/dev/null | grep -iE "(finstack|Raw|Gzip)" || true
	@printf "\nfinstack-core contribution in Python binary:\n"
	@$(MAKE) size-py 2>/dev/null | grep -E "(finstack-core|File|Compressed)" || true
size-all: size-wasm size-py

# --- Package Building ---

WHEEL_DIR := target/wheels

.PHONY: wheel-local wheel-docker wheel-all wasm-pkg wasm-publish-dry

wheel-local: ## Build wheel for current platform + Python
	@printf "Building wheel for local platform...\n"
	@$(call py_run,maturin build --release \
		--manifest-path finstack-py/Cargo.toml \
		-o $(WHEEL_DIR))
	@printf "Wheel(s) written to $(WHEEL_DIR)/\n"
	@ls -lh $(WHEEL_DIR)/finstack-*.whl

wheel-docker: ## Build manylinux wheel via Docker (current arch)
	@printf "Building manylinux wheel via Docker...\n"
	MSYS_NO_PATHCONV=1 docker run --rm \
		-v "$(CURDIR):/io" \
		-w /io \
		ghcr.io/pyo3/maturin:v1.10 \
		build --release \
		--manifest-path finstack-py/Cargo.toml \
		-o /io/$(WHEEL_DIR)
	@printf "Wheel(s) written to $(WHEEL_DIR)/\n"
	@ls -lh $(WHEEL_DIR)/finstack-*.whl

wheel-all: ## Build wheels for all locally-available Python versions
	@printf "Building wheels for all available Python interpreters...\n"
	@$(call py_run,maturin build --release \
		--manifest-path finstack-py/Cargo.toml \
		--find-interpreter \
		-o $(WHEEL_DIR))
	@printf "Wheel(s) written to $(WHEEL_DIR)/\n"
	@ls -lh $(WHEEL_DIR)/finstack-*.whl

wasm-pkg: ## Build WASM package (web + node targets)
	cd finstack-wasm && wasm-pack build --target web --release --out-dir pkg
	cd finstack-wasm && wasm-pack build --target nodejs --release --out-dir pkg-node
	touch finstack-wasm/pkg/.npmignore finstack-wasm/pkg-node/.npmignore

wasm-publish-dry: wasm-pkg ## Dry-run npm publish (no upload)
	cd finstack-wasm && npm pack --dry-run --ignore-scripts

# --- Automation & CI ---

.PHONY: ci-test
ci-test: wasm-build pre-commit-run test ## Run all checks exactly as CI does (wasm-build + pre-commit + test)

# --- Pre-commit ---

.PHONY: pre-commit-install pre-commit-run pre-commit-update
pre-commit-install: install-deny
	@if [ ! -d "$(VENV)" ]; then uv venv; fi
	@$(call py_run,uv pip install pre-commit && pre-commit install && pre-commit install --hook-type pre-push)

pre-commit-run: install-deny
	@$(call py_run,pre-commit run --all-files)
	@$(call py_run,pre-commit run --all-files --hook-stage pre-push)

pre-commit-update:
	@$(call py_run,pre-commit autoupdate)

# --- Tooling Installation ---

.PHONY: install-nextest install-mdbook install-bloat install-twiggy install-deny
install-nextest:
	@command -v cargo-nextest >/dev/null 2>&1 || cargo install cargo-nextest --locked
install-mdbook:
	@command -v mdbook >/dev/null 2>&1 || cargo install mdbook
	@command -v mdbook-jupyter >/dev/null 2>&1 || cargo install mdbook-jupyter
install-bloat:
	@command -v cargo-bloat >/dev/null 2>&1 || cargo install cargo-bloat --locked
install-twiggy:
	@command -v twiggy >/dev/null 2>&1 || cargo install twiggy
install-deny:
	@command -v cargo-deny >/dev/null 2>&1 || cargo install cargo-deny --locked

# --- Cleanup ---

.PHONY: clean clean-cache
clean: ## Remove build artifacts
	cargo clean
	rm -rf $(VENV)
	rm -rf finstack-wasm/pkg finstack-wasm/pkg-node
	rm -rf book/book
	find . -name "__pycache__" -type d -exec rm -rf {} + 2>/dev/null || true
	find . -name "*.egg-info" -type d -exec rm -rf {} + 2>/dev/null || true

clean-cache: ## Clear tool caches (ruff, ty, pytest)
	rm -rf .ruff_cache .ty_cache .mypy_cache .pytest_cache
	rm -rf finstack-py/.ruff_cache finstack-py/.ty_cache finstack-py/.mypy_cache finstack-py/.pytest_cache
	@printf "Caches cleared.\n"
