.PHONY: help setup-python build build-prod test-rust test-rust-slow test-rust-doc test-python doc clean fmt lint stubs coverage coverage-html coverage-open coverage-lcov wasm-examples-dev examples ci_test install-nextest book-build book-serve book-clean book-watch install-mdbook bench-perf bench-baseline bench-flamegraph bench-compare install-bloat size-wasm size-py size-core size-all

help:
	@echo "Builds:"
	@echo "  build         				- Build all crates"
	@echo "  build-prod    				- Build all crates optimized without debug info"
	@echo "  python-dev    				- Build Python bindings in development mode"
	@echo "  wasm-build    				- Build WASM package"
	@echo "  examples       				- Run all Rust examples"
	@echo ""
	@echo "Formatting:"
	@echo "  fmt-rust       				- Format Rust code"
	@echo "  fmt-python     				- Format Python code"
	@echo "  fmt-wasm       				- Format WASM code"
	@echo ""
	@echo "Linting:"
	@echo "  lint-rust      				- Run Rust linters"
	@echo "  lint-python    				- Run Python linters"
	@echo "  lint-wasm      				- Run WASM linters"
	@echo ""
	@echo "Linting fixes:"
	@echo "  lint-rust-fix      				- Run Rust linters fixes"
	@echo "  lint-python-fix   				- Run Python linters fixes"
	@echo "  lint-wasm-fix      				- Run WASM linters fixes"
	@echo ""
	@echo "Testing:"
	@echo "  test-rust      				- Run Rust tests (cargo-nextest)"
	@echo "  test-rust-slow 				- Run all Rust tests incl. slow (cargo-nextest)"
	@echo "  test-rust-doc  				- Run Rust documentation tests only"
	@echo "  test-python     				- Run Python tests in finstack-py"
	@echo "  test-wasm       				- Run WASM tests in finstack-wasm"
	@echo ""
	@echo "Benchmarking & Profiling:"
	@echo "  bench-perf         				- Run all benchmarks with optimized profile"
	@echo "  bench-baseline     				- Save benchmark baseline for comparison"
	@echo "  bench-compare      				- Compare benchmarks against baseline"
	@echo "  bench-flamegraph   				- Generate CPU flamegraph for MC pricing"
	@echo ""
	@echo "Binary Size Analysis:"
	@echo "  install-bloat      				- Install cargo-bloat tool for size analysis"
	@echo "  size-wasm          				- Analyze WASM binary size by crate"
	@echo "  size-py            				- Analyze Python bindings binary size by crate"
	@echo "  size-core          				- Show finstack-core contribution in binaries"
	@echo "  size-all           				- Analyze all binaries (WASM, Python)"
	@echo ""
	@echo "Documentation:"
	@echo "  doc            				- Generate rustdoc documentation (workspace crates only, no deps)"
	@echo "  book-build     				- Build mdBook documentation"
	@echo "  book-serve     				- Build and serve mdBook with live reload"
	@echo "  book-watch     				- Watch and rebuild mdBook on changes"
	@echo "  book-clean     				- Clean mdBook build artifacts"
	@echo ""
	@echo "Other:"
	@echo "  clean          				- Clean build artifacts"
	@echo "  setup-python  				- Set up Python development environment with uv"
	@echo "  install-nextest  				- Install cargo-nextest (test runner)"
	@echo "  install-mdbook  				- Install mdBook (documentation builder)"
	@echo "  ci_test       				- Run all CI checks locally (mirrors GitHub Actions)"

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

test:
	make test-rust
	make test-rust-doc
	make test-python
	make test-wasm
	make test-ui

test-rust: install-nextest
	CARGO_INCREMENTAL=1 cargo nextest run --workspace --exclude finstack-py --features mc --lib --test '*' --no-fail-fast

test-rust-slow: install-nextest
	CARGO_INCREMENTAL=1 cargo nextest run --workspace --exclude finstack-py --features mc,slow --lib --test '*'

check-no-doctest-ignore:
	@set -e; \
	if rg -n '^[[:space:]]*```[^\n]*\bignore\b' --glob '**/*.rs' ; then \
		echo "ERROR: Found doctest code fences using 'ignore'."; \
		echo "Use 'rust,no_run' for compile-only examples, 'rust' for runnable examples, or 'text' for non-Rust snippets."; \
		exit 1; \
	fi

test-rust-doc: check-no-doctest-ignore
	CARGO_INCREMENTAL=1 cargo test --workspace --exclude finstack-py --doc --features mc

test-python:
	@command -v uv >/dev/null 2>&1 || { echo "uv is required for Python tests (https://github.com/astral-sh/uv)."; exit 1; }
	cd finstack-py && uv run pytest tests -v

test-wasm:
	cd finstack-wasm && npm run test

test-ui:
	cd packages/finstack-ui && npm run test -- run

test-ui-coverage:
	cd packages/finstack-ui && npm run test:coverage

fmt-rust:
	cargo fmt --all

fmt-python:
	cd finstack-py && uv run ruff format .

fmt-wasm:
	cd finstack-wasm && npm run format .

fmt-ui:
	cd packages/finstack-ui && npm run format:fix .

lint:
	make lint-rust
	make lint-python
	make lint-wasm
	make lint-ui

lint-rust:
	PYO3_PYTHON=python3 CARGO_INCREMENTAL=1 cargo clippy --workspace --all-targets --all-features -- -D warnings

lint-python:
	@if command -v uv >/dev/null 2>&1; then \
		if [ -f .venv/bin/activate ]; then \
			. .venv/bin/activate && ruff check .; \
		else \
			uv run ruff check .; \
		fi \
	fi

lint-python-fix:
	@if command -v uv >/dev/null 2>&1; then \
		if [ -f .venv/bin/activate ]; then \
			. .venv/bin/activate && ruff check . --fix; \
		else \
			uv run ruff check . --fix; \
		fi \
	fi

lint-wasm:
	cd finstack-wasm && npm run lint

lint-ui:
	cd packages/finstack-ui && npm run lint

lint-wasm-fix:
	cd finstack-wasm && npm run lint:fix

lint-ui-fix:
	cd packages/finstack-ui && npm run lint:fix

clean:
	cargo clean
	rm -rf .venv
	rm -rf finstack-wasm/pkg
	rm -rf finstack-wasm/pkg-node
	rm -rf book/book
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
		CARGO_INCREMENTAL=1 python -m maturin develop --profile release-perf

wasm-build:
	cd finstack-wasm && wasm-pack build --target web

wasm-examples-dev: wasm-build
	cd finstack-wasm && \
	npm run examples:install && \
	npm run examples:dev

examples:
	@echo "════════════════════════════════════════════════════════════════"
	@echo "🚀 Running all Rust examplesx"
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

# Performance profiling targets
bench-perf:
	@echo "Running benchmarks with performance profile..."
	cargo bench --profile bench

bench-baseline:
	@echo "Saving benchmark baseline..."
	cargo bench -- --save-baseline main

bench-flamegraph:
	@echo "Generating flamegraph for MC pricing benchmark..."
	@command -v flamegraph >/dev/null 2>&1 || { echo "Installing flamegraph..."; cargo install flamegraph; }
	cargo flamegraph --bench mc_pricing --profile bench --features mc -- --bench

bench-compare:
	@echo "Comparing benchmarks against baseline..."
	cargo bench -- --baseline main

doc:
	CARGO_INCREMENTAL=1 cargo doc --workspace --exclude finstack-py --exclude finstack-wasm --no-deps --all-features --open

install-nextest:
	@if command -v cargo-nextest >/dev/null 2>&1; then \
		echo "cargo-nextest already installed"; \
	else \
		echo "Installing cargo-nextest..."; \
		cargo install cargo-nextest --locked; \
	fi

install-mdbook:
	@if command -v mdbook >/dev/null 2>&1; then \
		echo "mdbook already installed"; \
	else \
		echo "Installing mdbook..."; \
		cargo install mdbook; \
	fi

book-build: install-mdbook
	@echo "Building mdBook documentation..."
	cd book && mdbook build

book-serve: install-mdbook
	@echo "Building and serving mdBook with live reload..."
	@echo "Documentation will be available at http://localhost:3000"
	cd book && mdbook serve --open

book-watch: install-mdbook
	@echo "Watching for changes and rebuilding mdBook..."
	cd book && mdbook watch

book-clean:
	@echo "Cleaning mdBook build artifacts..."
	rm -rf book/book

check-dups:
	@echo "Checking for duplicate code..."
	npx jscpd --pattern "**/src/**/*.rs" --ignore "**/target/**,**/node_modules/**,**/tests/**"

# Binary size analysis targets
install-bloat:
	@if command -v cargo-bloat >/dev/null 2>&1; then \
		echo "cargo-bloat already installed"; \
	else \
		echo "Installing cargo-bloat..."; \
		cargo install cargo-bloat --locked; \
	fi

size-wasm: install-bloat
	@echo "Analyzing WASM binary size..."
	@echo "Building WASM binary first..."
	cd finstack-wasm && wasm-pack build --target web --release
	@echo ""
	@echo "=== WASM Binary Size Analysis (by crate) ==="
	cargo bloat --release --crates -p finstack-wasm --target wasm32-unknown-unknown
	@echo ""
	@echo "=== WASM Binary Size Analysis (by function) ==="
	cargo bloat --release --functions -p finstack-wasm --target wasm32-unknown-unknown | head -50

size-py: install-bloat
	@echo "Analyzing Python bindings binary size..."
	@echo "Building Python bindings first..."
	cd finstack-py && cargo build --release
	@echo ""
	@echo "=== Python Bindings Binary Size Analysis (by crate) ==="
	cargo bloat --release --crates -p finstack-py
	@echo ""
	@echo "=== Python Bindings Binary Size Analysis (by function) ==="
	cargo bloat --release --functions -p finstack-py | head -50

size-core: install-bloat
	@echo "Analyzing finstack-core contribution in binaries..."
	@echo "Note: Library crates (rlib) cannot be analyzed directly."
	@echo "Showing finstack-core size contribution in binaries that use it..."
	@echo ""
	@echo "=== finstack-core in WASM binary ==="
	@if [ -f finstack-wasm/target/wasm32-unknown-unknown/release/finstack_wasm.wasm ]; then \
		cargo bloat --release --crates -p finstack-wasm --target wasm32-unknown-unknown | grep -E "(finstack-core|File|Compressed)" || echo "Build WASM first with: make size-wasm"; \
	else \
		echo "WASM not built. Building now..."; \
		cd finstack-wasm && wasm-pack build --target web --release 2>/dev/null || true; \
		cargo bloat --release --crates -p finstack-wasm --target wasm32-unknown-unknown 2>/dev/null | grep -E "(finstack-core|File|Compressed)" || echo "Could not analyze WASM"; \
	fi
	@echo ""
	@echo "=== finstack-core in Python bindings ==="
	@if [ -f finstack-py/target/release/libfinstack*.so ] || [ -f finstack-py/target/release/libfinstack*.dylib ] || [ -f finstack-py/target/release/finstack*.dll ]; then \
		cargo bloat --release --crates -p finstack-py 2>/dev/null | grep -E "(finstack-core|File|Compressed)" || echo "Could not analyze Python bindings"; \
	else \
		echo "Python bindings not built. Building now..."; \
		cd finstack-py && cargo build --release 2>/dev/null || true; \
		cargo bloat --release --crates -p finstack-py 2>/dev/null | grep -E "(finstack-core|File|Compressed)" || echo "Could not analyze Python bindings"; \
	fi

size-all: size-wasm size-py
	@echo ""
	@echo "=== Summary: Binary sizes ==="
	@echo "WASM binary:"
	@ls -lh finstack-wasm/pkg/finstack_wasm_bg.wasm 2>/dev/null || echo "  (not built)"
	@echo "Python bindings:"
	@find finstack-py -name "*.so" -o -name "*.dylib" -o -name "*.dll" 2>/dev/null | xargs ls -lh 2>/dev/null || echo "  (not built)"
	@echo ""
	@echo "Note: Library crates (rlib) like finstack-core cannot be analyzed directly."
	@echo "Use 'make size-core' to see finstack-core contribution in binaries."