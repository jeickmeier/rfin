.PHONY: help setup-python build build-prod test test-slow test-doc clean fmt lint stubs coverage coverage-html coverage-open coverage-lcov wasm-examples-dev examples ci_test install-nextest

help:
	@echo "Available targets:"
	@echo "  setup-python  - Set up Python development environment with uv"
	@echo "  build         - Build all crates"
	@echo "  build-prod    - Build all crates optimized without debug info"
	@echo ""
	@echo "Testing:"
	@echo "  test           - Run all tests (cargo-nextest)"
	@echo "  test-slow      - Run all tests incl. slow (cargo-nextest)"
	@echo "  test-doc       - Run documentation tests only"
	@echo ""
	@echo "Other:"
	@echo "  fmt            - Format all code"
	@echo "  lint           - Run linters"
	@echo "  clean          - Clean build artifacts"
	@echo "  install-nextest  - Install cargo-nextest (test runner)"
	@echo "  python-dev    - Build Python bindings in development mode"
	@echo "  stubs         - Regenerate *.pyi stub files for VS Code IntelliSense"
	@echo "  wasm-build    - Build WASM package"
	@echo "  wasm-examples-dev - Build WASM, then start examples dev server"
	@echo "  examples      - Run all Rust examples"
	@echo "  coverage      - Run code coverage and print summary"
	@echo "  coverage-html - Generate HTML coverage report"
	@echo "  coverage-open - Generate HTML coverage report and open in browser"
	@echo "  coverage-lcov - Generate LCOV coverage report for CI"
	@echo "  ci_test       - Run all CI checks locally (mirrors GitHub Actions)"

setup-python:
	@echo "Setting up Python development environment..."
	uv venv
	@echo "Virtual environment created. Now run:"
	@echo "  source .venv/bin/activate"
	@echo "  make python-dev"

build:
	CARGO_INCREMENTAL=1 cargo build --workspace --exclude finstack-py --exclude finstack-wasm

build-prod:
	CARGO_INCREMENTAL=1 RUSTFLAGS="-C debuginfo=0" cargo build --workspace --exclude finstack-py --exclude finstack-wasm --release

test: install-nextest
	CARGO_INCREMENTAL=1 cargo nextest run --workspace --exclude finstack-py --features mc

test-slow: install-nextest
	CARGO_INCREMENTAL=1 cargo nextest run --workspace --exclude finstack-py --features mc,slow

test-doc:
	CARGO_INCREMENTAL=1 cargo test --workspace --exclude finstack-py --doc --features mc

install-nextest:
	@if command -v cargo-nextest >/dev/null 2>&1; then \
		echo "cargo-nextest already installed"; \
	else \
		echo "Installing cargo-nextest..."; \
		cargo install cargo-nextest --locked; \
	fi

fmt:
	cargo fmt --all

lint:
	PYO3_PYTHON=python3 CARGO_INCREMENTAL=1 cargo clippy --workspace --all-targets --all-features  -- -D warnings
	@if command -v uv >/dev/null 2>&1; then \
		if [ -f .venv/bin/activate ]; then \
			. .venv/bin/activate && ruff check .; \
		else \
			uv run ruff check .; \
		fi \
	fi

clean:
	cargo clean
	rm -rf .venv
	rm -rf finstack-wasm/pkg
	rm -rf finstack-wasm/pkg-node
	find . -name "__pycache__" -type d -exec rm -rf {} + 2>/dev/null || true
	find . -name "*.egg-info" -type d -exec rm -rf {} + 2>/dev/null || true


python-dev:
	@if [ ! -d ".venv" ]; then \
		echo "Virtual environment not found. Creating one..."; \
		uv venv; \
	fi
	@echo "Installing Python dependencies and building extension..."
	. .venv/bin/activate && \
	uv pip install maturin pytest pytest-benchmark black mypy ruff ipython jupyter && \
	cd finstack-py && \
		CARGO_INCREMENTAL=1 python -m maturin develop --release

wasm-build:
	cd finstack-wasm && wasm-pack build --target web

wasm-examples-dev: wasm-build
	cd finstack-wasm && \
	npm run examples:install && \
	npm run examples:dev

examples:
	@echo "════════════════════════════════════════════════════════════════"
	@echo "🚀 Running all Rust examples"
	@echo "════════════════════════════════════════════════════════════════"
	@echo ""
	@command -v jq >/dev/null 2>&1 || { echo "❌ jq is required but not installed. Install with: brew install jq"; exit 1; }
	@example_list=$$(cargo metadata --format-version=1 --no-deps | jq -r '.packages[] | select(.name == "finstack") | .targets[] | select(.kind[] == "example") | .name'); \
	last_category=""; \
	for example in $$example_list; do \
		category=""; \
		if echo "$$example" | grep -q "^market_context"; then category="Core"; \
		elif echo "$$example" | grep -q "portfolio"; then category="Portfolio"; \
		elif echo "$$example" | grep -q "scenario"; then category="Scenarios"; \
		elif echo "$$example" | grep -q "^statements\|^capital_structure\|^lbo_"; then category="Statements"; \
		else category="Valuations"; fi; \
		if [ "$$category" != "$$last_category" ]; then \
			echo ""; \
			echo "📋 $$category Examples"; \
			echo "────────────────────────────────────────────────────────────────"; \
			last_category="$$category"; \
		fi; \
		echo "Running $$example..."; \
		CARGO_INCREMENTAL=1 cargo run --example $$example --all-features || exit 1; \
		echo ""; \
	done
	@echo "════════════════════════════════════════════════════════════════"
	@echo "🎉 All examples completed successfully!"
	@echo "════════════════════════════════════════════════════════════════"

stubs:
	@echo "(re)generating Python stub files …"
	bash ./scripts/generate-stubs.sh
	@echo "Stub generation complete."

coverage:
	@echo "Running code coverage (finstack Rust library only)..."
	CARGO_INCREMENTAL=1 cargo llvm-cov --workspace --exclude finstack-py --exclude finstack-wasm --ignore-filename-regex '(tests?/|target/|\.cargo/|.*finstack-py/.*|.*finstack-wasm/.*)'

coverage-html:
	@echo "Generating HTML coverage report (finstack Rust library only)..."
	CARGO_INCREMENTAL=1 cargo llvm-cov --workspace --exclude finstack-py --exclude finstack-wasm --html --ignore-filename-regex '(tests?/|target/|\.cargo/|.*finstack-py/.*|.*finstack-wasm/.*)'

coverage-open:
	@echo "Generating and opening HTML coverage report (finstack Rust library only)..."
	CARGO_INCREMENTAL=1 cargo llvm-cov --workspace --exclude finstack-py --exclude finstack-wasm --html --open --ignore-filename-regex '(tests?/|target/|\.cargo/|.*finstack-py/.*|.*finstack-wasm/.*)'

coverage-lcov:
	@echo "Generating LCOV coverage report for CI (finstack Rust library only)..."
	CARGO_INCREMENTAL=1 cargo llvm-cov --workspace --exclude finstack-py --exclude finstack-wasm --lcov --output-path coverage.lcov --ignore-filename-regex '(tests?/|target/|\.cargo/|.*finstack-py/.*|.*finstack-wasm/.*)'
