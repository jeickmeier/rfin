# RFin Development Makefile
# -------------------------
# A developer-friendly entry point for building, testing, and linting the RFin project.

# --- Configuration & Macros ---

.DEFAULT_GOAL := help
SHELL := /bin/bash

# Detect Python environment
VENV := .venv
VENV_PATH := $(CURDIR)/$(VENV)
PYTHON_ACTIVATE := [ -f $(VENV_PATH)/bin/activate ] && . $(VENV_PATH)/bin/activate
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
	@printf "  \033[36mlint\033[0m                Check for linting issues (without fixing)\n"
	@printf "  \033[36mci-test\033[0m             Run all checks as they would run in CI\n\n"
	@printf "Component Specifics:\n"
	@printf "  \033[36mtest-rust\033[0m           Run Rust tests (cargo-nextest)\n"
	@printf "  \033[36mtest-python\033[0m         Run Python tests\n"
	@printf "  \033[36mexamples-python\033[0m     Run all Python examples (scripts & notebooks)\n"
	@printf "  \033[36mtest-wasm\033[0m           Run WASM package tests\n"
	@printf "  \033[36mtest-ui\033[0m             Run UI component tests\n\n"
	@printf "Setup & Maintenance:\n"
	@printf "  \033[36msetup-python\033[0m        Initialize Python environment with uv\n"
	@printf "  \033[36mpython-dev\033[0m          Install Python deps and build bindings\n"
	@printf "  \033[36mtest-and-fix\033[0m        Run all tests and attempt auto-fixes\n"
	@printf "  \033[36mclean\033[0m               Remove build artifacts and virtualenvs\n\n"
	@printf "Documentation:\n"
	@printf "  \033[36mdoc\033[0m                 Generate Rust documentation\n"
	@printf "  \033[36mbook-serve\033[0m          Serve mdBook with live reload\n\n"
	@printf "Development & Tooling:\n"
	@printf "  \033[36mdev-ui\033[0m              Start UI development server (Vite)\n"
	@printf "  \033[36mgenerate-bindings\033[0m   Export TypeScript types from Rust\n"
	@printf "  \033[36mexamples-python-scripts\033[0m   Run Python example scripts\n"
	@printf "  \033[36mexamples-python-notebooks\033[0m Run Python example notebooks\n"
	@printf "  \033[36mcheck-env\033[0m           Verify development environment\n"
	@printf "  \033[36mupdate\033[0m              Update all dependencies (Rust, Python, JS)\n"
	@printf "  \033[36maudit\033[0m               Run security audits on all components\n\n"
	@printf "Analysis & Coverage:\n"
	@printf "  \033[36mcoverage\033[0m            Run coverage for all components\n"
	@printf "  \033[36mcoverage-rust\033[0m       Run Rust code coverage\n"
	@printf "  \033[36mcoverage-python\033[0m     Run Python code coverage\n"
	@printf "  \033[36mcoverage-ui\033[0m         Run UI code coverage\n"
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
test: test-rust test-rust-doc test-python test-wasm test-ui ## Run all tests across all components

.PHONY: fmt
fmt: ## Format all code (Rust, Python, WASM, UI, MD)
	./scripts/format-code

.PHONY: lint
lint: ## Check all code for linting issues
	./scripts/format-code --check-only

# --- Component: Rust ---

.PHONY: test-rust
test-rust: install-nextest
	CARGO_INCREMENTAL=1 cargo nextest run --workspace --exclude finstack-py --features mc,test-utils --lib --test '*' --no-fail-fast

.PHONY: test-rust-slow
test-rust-slow: install-nextest
	CARGO_INCREMENTAL=1 cargo nextest run --workspace --exclude finstack-py --features mc,slow,test-utils --lib --test '*'

.PHONY: test-rust-doc
test-rust-doc: check-no-doctest-ignore
	CARGO_INCREMENTAL=1 cargo test --workspace --exclude finstack-py --doc --features mc

.PHONY: fmt-rust
fmt-rust:
	./scripts/format-code --rust-only

.PHONY: lint-rust
lint-rust:
	./scripts/format-code --rust-only --check-only

.PHONY: lint-rust-fix
lint-rust-fix:
	./scripts/format-code --rust-only

.PHONY: check-no-doctest-ignore
check-no-doctest-ignore:
	@set -e; \
	if rg -n '^[[:space:]]*```[^\n]*\bignore\b' --glob '**/*.rs' ; then \
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
	@printf "Virtual environment created. Now run: source .venv/bin/activate && make python-dev\n"

.PHONY: python-dev
python-dev: ## Install dependencies and build bindings
	@if [ ! -d "$(VENV)" ]; then uv venv; fi
	@printf "Installing Python dependencies and building extension...\n"
	@$(call py_run,uv pip install maturin pytest pytest-benchmark black mypy ruff ipython jupyter)
	@cd finstack-py && $(call py_run,python -m maturin develop --profile release-perf)

.PHONY: test-python
test-python: ## Run Python tests
	@cd finstack-py && $(call py_run,pytest tests -v)

.PHONY: fmt-python
fmt-python:
	./scripts/format-code --python-only

.PHONY: lint-python
lint-python:
	./scripts/format-code --python-only --check-only

.PHONY: lint-python-fix
lint-python-fix:
	./scripts/format-code --python-only

.PHONY: typecheck-python
typecheck-python:
	@$(call py_run,pyright)

.PHONY: verifytypes-python
verifytypes-python:
	@$(call py_run,pyright --verifytypes finstack --ignoreexternal)

.PHONY: stubtest-python
stubtest-python:
	@printf "Use 'make verifytypes-python' for CI-grade type verification.\n"
	@printf "Local: uv run python -m mypy.stubtest finstack --ignore-missing-stub --allowlist finstack-py/tests/stubtest_allowlist.txt\n"

.PHONY: stubs
stubs:
	@printf "Python stubs (.pyi) are manually maintained in finstack-py/finstack/.\n"

.PHONY: examples-python examples-python-scripts examples-python-notebooks
examples-python: examples-python-scripts examples-python-notebooks ## Run all Python examples

examples-python-scripts: ## Run all Python example scripts
	@printf "Running Python example scripts...\n"
	@$(call py_run,python finstack-py/examples/scripts/run_all_examples.py)

examples-python-notebooks: ## Run all Python example notebooks
	@printf "Running Python example notebooks...\n"
	@$(call py_run,python finstack-py/examples/notebooks/run_all_notebooks.py)

# --- Component: WASM & UI ---

.PHONY: wasm-build
wasm-build: ## Build WASM package
	cd finstack-wasm && wasm-pack build --target web

.PHONY: wasm-examples-dev
wasm-examples-dev: wasm-build
	cd finstack-wasm && npm run examples:install && npm run examples:dev

.PHONY: test-wasm
test-wasm:
	cd finstack-wasm && npm run test

.PHONY: fmt-wasm
fmt-wasm:
	./scripts/format-code --wasm-only

.PHONY: lint-wasm
lint-wasm:
	./scripts/format-code --wasm-only --check-only

.PHONY: lint-wasm-fix
lint-wasm-fix:
	./scripts/format-code --wasm-only

.PHONY: test-ui
test-ui:
	cd finstack-ui && npm run test -- run

.PHONY: dev-ui
dev-ui: ## Start UI development server
	cd finstack-ui && npm run dev

.PHONY: generate-bindings
generate-bindings: ## Export TypeScript types from Rust
	@printf "Generating TypeScript bindings...\n"
	cargo run -p finstack-wasm --bin ts_export --features ts_export

.PHONY: test-ui-coverage
test-ui-coverage:
	cd finstack-ui && npm run test:coverage

.PHONY: fmt-ui
fmt-ui:
	./scripts/format-code --ui-only

.PHONY: lint-ui
lint-ui:
	./scripts/format-code --ui-only --check-only

.PHONY: lint-ui-fix
lint-ui-fix:
	./scripts/format-code --ui-only

# --- Documentation ---

.PHONY: doc
doc: ## Generate and open rustdoc
	CARGO_INCREMENTAL=1 cargo doc --workspace --exclude finstack-py --exclude finstack-wasm --no-deps --all-features --open

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

.PHONY: coverage coverage-rust coverage-python coverage-ui coverage-html coverage-open coverage-lcov
coverage: coverage-rust coverage-python coverage-ui ## Run all coverage reports

coverage-rust:
	@printf "Running Rust code coverage...\n"
	CARGO_INCREMENTAL=1 cargo llvm-cov --workspace --exclude finstack-py --exclude finstack-wasm --ignore-filename-regex '(tests?/|target/|\.cargo/|.*finstack-py/.*|.*finstack-wasm/.*)'

coverage-python:
	@printf "Running Python code coverage...\n"
	@cd finstack-py && $(call py_run,pytest --cov=finstack --cov-report=html tests)

coverage-ui:
	@printf "Running UI code coverage...\n"
	cd finstack-ui && npm run test:coverage

coverage-html:
	CARGO_INCREMENTAL=1 cargo llvm-cov --workspace --exclude finstack-py --exclude finstack-wasm --html --ignore-filename-regex '(tests?/|target/|\.cargo/|.*finstack-py/.*|.*finstack-wasm/.*)'

coverage-open:
	CARGO_INCREMENTAL=1 cargo llvm-cov --workspace --exclude finstack-py --exclude finstack-wasm --html --open --ignore-filename-regex '(tests?/|target/|\.cargo/|.*finstack-py/.*|.*finstack-wasm/.*)'

coverage-lcov:
	CARGO_INCREMENTAL=1 cargo llvm-cov --workspace --exclude finstack-py --exclude finstack-wasm --lcov --output-path coverage.lcov --ignore-filename-regex '(tests?/|target/|\.cargo/|.*finstack-py/.*|.*finstack-wasm/.*)'

.PHONY: check-schemas
check-schemas: ## Verify JSON schemas match Rust types
	cargo nextest run -p finstack-valuations schema_parity --no-fail-fast
	cargo test -p finstack-valuations test_instrument_schema_enum_parity --no-fail-fast

.PHONY: check-dups
check-dups:
	npx jscpd --pattern "**/src/**/*.rs" --ignore "**/target/**,**/node_modules/**,**/tests/**"

.PHONY: audit audit-rust audit-python audit-ui
audit: audit-rust audit-python audit-ui ## Run security audits on all components
audit-rust:
	@printf "Auditing Rust dependencies...\n"
	@command -v cargo-audit >/dev/null 2>&1 || { printf "cargo-audit not found. Install with: cargo install cargo-audit\n"; exit 1; }
	cargo audit
audit-python:
	@printf "Auditing Python dependencies...\n"
	@$(call py_run,bandit -r finstack-py -c pyproject.toml)
audit-ui:
	@printf "Auditing UI dependencies...\n"
	cd finstack-ui && npm audit

.PHONY: update update-rust update-python update-ui
update: update-rust update-python update-ui ## Update all dependencies
update-rust:
	@printf "Updating Rust dependencies...\n"
	cargo update
update-python:
	@printf "Updating Python dependencies...\n"
	uv lock --upgrade
update-ui:
	@printf "Updating UI dependencies...\n"
	cd finstack-ui && npm update

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
size-wasm: install-bloat
	cd finstack-wasm && wasm-pack build --target web --release
	cargo bloat --release --crates -p finstack-wasm --target wasm32-unknown-unknown
size-py: install-bloat
	cd finstack-py && cargo build --release
	cargo bloat --release --crates -p finstack-py
size-core: install-bloat
	@printf "finstack-core contribution in binaries:\n"
	@$(MAKE) size-wasm 2>/dev/null | grep -E "(finstack-core|File|Compressed)" || true
	@$(MAKE) size-py 2>/dev/null | grep -E "(finstack-core|File|Compressed)" || true
size-all: size-wasm size-py

# --- Automation & CI ---

.PHONY: test-and-fix test-fix-rust test-fix-python test-fix-wasm test-fix-ui
test-and-fix: ## Run all tests and auto-fix issues
	./scripts/run-tests-and-fix
test-fix-rust:
	./scripts/run-tests-and-fix --rust-only
test-fix-python:
	./scripts/run-tests-and-fix --python-only
test-fix-wasm:
	./scripts/run-tests-and-fix --wasm-only
test-fix-ui:
	./scripts/run-tests-and-fix --ui-only

.PHONY: ci-test
ci-test: ## Run local CI checks
	./scripts/run-tests-and-fix --skip-slow

# --- Pre-commit ---

.PHONY: pre-commit-install pre-commit-run pre-commit-update
pre-commit-install:
	@if [ ! -d "$(VENV)" ]; then uv venv; fi
	@$(call py_run,uv pip install pre-commit && pre-commit install && pre-commit install --hook-type pre-push)

pre-commit-run:
	@$(call py_run,pre-commit run --all-files)

pre-commit-update:
	@$(call py_run,pre-commit autoupdate)

# --- Tooling Installation ---

.PHONY: install-nextest install-mdbook install-bloat
install-nextest:
	@command -v cargo-nextest >/dev/null 2>&1 || cargo install cargo-nextest --locked
install-mdbook:
	@command -v mdbook >/dev/null 2>&1 || cargo install mdbook
install-bloat:
	@command -v cargo-bloat >/dev/null 2>&1 || cargo install cargo-bloat --locked

# --- Cleanup ---

.PHONY: clean clean-cache
clean: ## Remove build artifacts
	cargo clean
	rm -rf $(VENV)
	rm -rf finstack-wasm/pkg finstack-wasm/pkg-node
	rm -rf book/book
	find . -name "__pycache__" -type d -exec rm -rf {} + 2>/dev/null || true
	find . -name "*.egg-info" -type d -exec rm -rf {} + 2>/dev/null || true

clean-cache: ## Clear tool caches (ruff, mypy, pytest)
	rm -rf .ruff_cache .mypy_cache .pytest_cache
	rm -rf finstack-py/.ruff_cache finstack-py/.mypy_cache finstack-py/.pytest_cache
	@printf "Caches cleared.\n"
