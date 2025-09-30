.PHONY: help setup-python build test clean fmt lint stubs coverage coverage-html coverage-open coverage-lcov

help:
	@echo "Available targets:"
	@echo "  setup-python  - Set up Python development environment with uv"
	@echo "  build         - Build all crates"
	@echo "  test          - Run all tests"
	@echo "  fmt           - Format all code"
	@echo "  lint          - Run linters"
	@echo "  clean         - Clean build artifacts"
	@echo "  python-dev    - Build Python bindings in development mode"
	@echo "  stubs         - Regenerate *.pyi stub files for VS Code IntelliSense"
	@echo "  wasm-build    - Build WASM package"
	@echo "  coverage      - Run code coverage and print summary"
	@echo "  coverage-html - Generate HTML coverage report"
	@echo "  coverage-open - Generate HTML coverage report and open in browser"
	@echo "  coverage-lcov - Generate LCOV coverage report for CI"

setup-python:
	@echo "Setting up Python development environment..."
	uv venv
	@echo "Virtual environment created. Now run:"
	@echo "  source .venv/bin/activate"
	@echo "  make python-dev"

build:
	cargo build --workspace

test:
	cargo test --workspace

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

stubs:
	@echo "(re)generating Python stub files …"
	bash ./scripts/generate-stubs.sh
	@echo "Stub generation complete."

coverage:
	@echo "Running code coverage..."
	CARGO_INCREMENTAL=0 cargo llvm-cov --workspace --exclude finstack-py --exclude finstack-wasm --ignore-filename-regex '(tests?/|target/|\.cargo/|.*finstack-py/.*|.*finstack-wasm/.*)'

coverage-html:
	@echo "Generating HTML coverage report..."
	CARGO_INCREMENTAL=0 cargo llvm-cov --workspace --exclude finstack-py --exclude finstack-wasm --html --ignore-filename-regex '(tests?/|target/|\.cargo/|.*finstack-py/.*|.*finstack-wasm/.*)'

coverage-open:
	@echo "Generating HTML coverage report and opening in browser..."
	CARGO_INCREMENTAL=0 cargo llvm-cov --workspace --exclude finstack-py --exclude finstack-wasm --open --ignore-filename-regex '(tests?/|target/|\.cargo/|.*finstack-py/.*|.*finstack-wasm/.*)'

coverage-lcov:
	@echo "Generating LCOV coverage report for CI..."
	CARGO_INCREMENTAL=0 cargo llvm-cov --workspace --exclude finstack-py --exclude finstack-wasm --lcov --ignore-filename-regex '(tests?/|target/|\.cargo/|.*finstack-py/.*|.*finstack-wasm/.*)'