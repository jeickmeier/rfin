# AGENTS.md

## Project Structure

- Multi-crate Rust workspace: `finstack/core`, `finstack/valuations`, `finstack/statements`, `finstack/scenarios`, `finstack/portfolio`
- Python bindings in `finstack-py/` (PyO3); WASM bindings in `finstack-wasm/` (wasm-bindgen)
- `finstack/core/src` is ~43K lines of Rust; `finstack-py/src` has ~234 Rust binding files
- `.pyi` stubs in `finstack-py/finstack/` are manually maintained; parity tests under `finstack-py/tests/parity`
- Example notebooks in `finstack-py/examples/notebooks/`; runner script: `run_all_notebooks.py`

## Build and Tooling

- `uv` is the Python package manager; use `uv run` when running Python functions
- Makefile targets: `make fmt`, `make lint`, `make test`, `make python-dev` (release profile), `make python-dev-debug` (fast compile)
- `make python-dev` uses `MATURIN_PROFILE=release` for runtime performance; debug builds are too slow for portfolio valuation
- Pre-commit runs `cargo clippy` and `cargo audit`
- Clippy runs with `-D warnings`; all warnings are treated as errors

## Clippy Strictness

- `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::panic)]`, `#![forbid(unsafe_code)]` in binding crate
- `too_many_arguments` threshold is 7; use a params struct for more
- `-D missing_docs` is enabled; all public struct fields need doc comments
- `doc_overindented_list_items`: list item continuations use 2-space indent, not aligned to preceding text
- Fix lint/type/test errors before resorting to `#[allow(...)]` as last resort

## Architecture: Binding Layer

- All logic stays in Rust core crates; Python bindings do only type conversion, wrapper construction, error mapping, and ergonomic helpers
- WASM bindings must expose identical functionality; any logic in Python must be reimplementable in WASM
- Wrapper pattern: `pub(crate) inner: RustType` with `from_inner()` constructor
- Error handling: centralized `map_error()` / `core_to_py()` in `errors.rs`; never use `.unwrap()` or `.expect()` in non-test binding code
- Module registration: every submodule sets `__all__` and `__doc__`; uses consistent `register()` function pattern
- Builder pattern: fluent chaining (e.g., `Builder.builder(id).field(val).build()`)

## API Conventions

- Accessors use `get_*` naming (e.g., `get_discount()`, `get_forward()`, `get_price()`)
- Metric keys are fully qualified: `bucketed_dv01::USD-OIS::10y`, `cs01::ACME-HZD`, `pv01::usd_ois`
- Z-spread CS01 for bonds uses instrument ID as key (e.g., `cs01::BOND_A`), not `z_spread`
- Bond CS01 without a hazard curve uses z-spread bump method (market convention)

## Workflow Preferences

- Preferred flow: Audit/Review → Plan → Implement (in that order)
- When a plan file exists: do NOT edit the plan file; do not recreate todos that already exist; mark todos as `in_progress` when starting each one
- User reports issues by pasting terminal output (clippy, cargo audit, test failures) rather than describing them
- When moving files, use `mv` in terminal and update all import references; then lint and format
