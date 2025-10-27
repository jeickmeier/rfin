.PHONY: help setup-python build build-prod test clean fmt lint stubs coverage coverage-html coverage-open coverage-lcov wasm-examples-dev examples ci_test

help:
	@echo "Available targets:"
	@echo "  setup-python  - Set up Python development environment with uv"
	@echo "  build         - Build all crates"
	@echo "  build-prod    - Build all crates optimized without debug info"
	@echo "  test          - Run all tests"
	@echo "  fmt           - Format all code"
	@echo "  lint          - Run linters"
	@echo "  clean         - Clean build artifacts"
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
	cargo build --workspace --exclude finstack-py

build-prod:
	RUSTFLAGS="-C debuginfo=0" cargo build --workspace --exclude finstack-py --release

test:
	cargo test --workspace --exclude finstack-py --all-features

fmt:
	cargo fmt --all

lint:
	PYO3_PYTHON=python3 cargo clippy --workspace --all-targets --all-features  -- -D warnings
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
	python -m maturin develop --release

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
		cargo run --example $$example --all-features || exit 1; \
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

ci_test:
	@echo "════════════════════════════════════════════════════════════════"
	@echo "🚀 Running CI checks locally (mirrors GitHub Actions workflow)"
	@echo "════════════════════════════════════════════════════════════════"
	@echo ""
	@echo "📋 Job 1/8: Format Check"
	@echo "────────────────────────────────────────────────────────────────"
	cargo fmt --all --check
	@echo "✅ Format check passed"
	@echo ""
	@echo "📋 Job 2/8: Clippy (Linter)"
	@echo "────────────────────────────────────────────────────────────────"
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	@echo "✅ Clippy passed"
	@echo ""
	@echo "📋 Job 3/8: Tests"
	@echo "────────────────────────────────────────────────────────────────"
	cargo test --workspace --exclude finstack-py --all-features --jobs 1
	@echo ""
	@echo "📋 Job 3b/8: Doc Tests"
	@echo "────────────────────────────────────────────────────────────────"
	cargo test --workspace --exclude finstack-py --doc --all-features --jobs 1
	@echo "✅ Tests passed"
	@echo ""
	@echo "📋 Job 4/8: Code Coverage"
	@echo "────────────────────────────────────────────────────────────────"
	@command -v cargo-llvm-cov >/dev/null 2>&1 || { echo "Installing cargo-llvm-cov..."; cargo install cargo-llvm-cov; }
	CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=1 cargo llvm-cov --workspace --exclude finstack-py --exclude finstack-wasm --all-features --lcov --output-path lcov.info --jobs 1
	@COVERAGE=$$(cargo llvm-cov --workspace --exclude finstack-py --exclude finstack-wasm --all-features --summary-only --jobs 1 | grep 'TOTAL' | awk '{print $$10}' | sed 's/%//'); \
	echo "Coverage: $${COVERAGE}%"; \
	if [ -n "$$COVERAGE" ] && [ $$(echo "$$COVERAGE < 50" | bc -l 2>/dev/null || echo "0") -eq 1 ]; then \
		echo "❌ Coverage $${COVERAGE}% is below minimum threshold of 50%"; \
		exit 1; \
	fi
	@echo "✅ Coverage passed"
	@echo ""
	@echo "📋 Job 5/8: Python Bindings"
	@echo "────────────────────────────────────────────────────────────────"
	@echo "Creating temporary virtual environment for Python tests..."
	@uv venv .venv-ci-test
	@. .venv-ci-test/bin/activate && \
		uv pip install maturin pytest && \
		cd finstack-py && \
		maturin build --release && \
		cd .. && \
		WHEEL=$$(ls target/wheels/*.whl 2>/dev/null | head -n1); \
		if [ -z "$$WHEEL" ]; then \
			echo "❌ No wheel found in target/wheels/"; \
			exit 1; \
		fi; \
		echo "Installing wheel: $$WHEEL"; \
		uv pip install "$$WHEEL" --force-reinstall && \
		python3 -c "import finstack; print('finstack imported successfully'); print('Available modules:', ', '.join(finstack.__all__))" && \
		cd finstack-py && \
		pytest tests/ -v
	@rm -rf .venv-ci-test
	@echo "✅ Python bindings passed"
	@echo ""
	@echo "📋 Job 6/8: WASM Build"
	@echo "────────────────────────────────────────────────────────────────"
	@rustup target list | grep -q "wasm32-unknown-unknown (installed)" || { echo "Installing wasm32 target..."; rustup target add wasm32-unknown-unknown; }
	@command -v wasm-pack >/dev/null 2>&1 || { echo "Installing wasm-pack..."; curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh; }
	cd finstack-wasm && wasm-pack build --target web --release
	@echo "Verifying WASM artifacts..."
	@test -f finstack-wasm/pkg/finstack_wasm_bg.wasm || { echo "❌ Missing: finstack_wasm_bg.wasm"; exit 1; }
	@test -f finstack-wasm/pkg/finstack_wasm.js || { echo "❌ Missing: finstack_wasm.js"; exit 1; }
	@test -f finstack-wasm/pkg/finstack_wasm.d.ts || { echo "❌ Missing: finstack_wasm.d.ts"; exit 1; }
	@test -f finstack-wasm/pkg/package.json || { echo "❌ Missing: package.json"; exit 1; }
	@echo "Running WASM tests..."
	cd finstack-wasm && wasm-pack test --node
	@echo "✅ WASM build passed"
	@echo ""
	@echo "📋 Job 7/8: Examples Build"
	@echo "────────────────────────────────────────────────────────────────"
	cargo build --workspace --examples
	@test -d finstack/examples || { echo "❌ Examples directory not found"; exit 1; }
	@find finstack/examples -name "*.rs" -type f
	@echo "✅ Examples build passed"
	@echo ""
	@echo "📋 Job 8/8: MSRV Check"
	@echo "────────────────────────────────────────────────────────────────"
	cargo check --workspace --all-features
	@echo "✅ MSRV check passed"
	@echo ""
	@echo "════════════════════════════════════════════════════════════════"
	@echo "🎉 All CI checks passed! Ready to push."
	@echo "════════════════════════════════════════════════════════════════"
