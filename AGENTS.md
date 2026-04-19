# AGENTS.md

## Project Structure

- Multi-crate Rust workspace: `finstack/core`, `finstack/analytics`, `finstack/valuations`, `finstack/statements`, `finstack/statements-analytics`, `finstack/scenarios`, `finstack/portfolio`, `finstack/margin`, `finstack/correlation`, `finstack/monte_carlo`
- Python bindings in `finstack-py/` (PyO3); WASM bindings in `finstack-wasm/` (wasm-bindgen)
- Python binding Rust code lives under `finstack-py/src/bindings/` (one subdirectory per crate domain)
- WASM binding Rust code lives under `finstack-wasm/src/api/` with a hand-written JS facade at `finstack-wasm/index.js`
- `.pyi` stubs in `finstack-py/finstack/` are derived from contract and binding code; parity tests under `finstack-py/tests/parity`
- Parity contract at repo root: `parity_contract.toml`; design spec at `docs/superpowers/specs/2026-04-10-rust-canonical-api-alignment-design.md`
- Example notebooks in `finstack-py/examples/notebooks/`; runner script: `run_all_notebooks.py`

## Build and Tooling

- `uv` is the Python package manager; use `uv run` when running Python functions
- Makefile targets: `make fmt`, `make lint`, `make test`, `make python-dev` (dev profile, fast compile), `make python-dev-release` (release; use for portfolio-scale benchmarks), `make python-dev-debug` (alias of `python-dev`)
- `make python-dev-release` uses `MATURIN_PROFILE=release` by default (override with `MATURIN_PROFILE`); release is slower to compile but faster at runtime
- Pre-commit runs `cargo clippy` and `cargo audit`
- Clippy runs with `-D warnings`; all warnings are treated as errors

## Clippy Strictness

- `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::panic)]`, `#![forbid(unsafe_code)]` in binding crate
- `too_many_arguments` threshold is 7; use a params struct for more
- `-D missing_docs` is enabled; all public struct fields need doc comments
- `doc_overindented_list_items`: list item continuations use 2-space indent, not aligned to preceding text
- Fix lint/type/test errors before resorting to `#[allow(...)]` as last resort

## Architecture: Binding Layer

- Rust is the canonical API design. Type and function names in Python/WASM must match Rust exactly (exceptions only for host-language collisions, e.g. WASM `FsDate` for JS `Date`)
- All logic stays in Rust crates; bindings do only type conversion, wrapper construction, error mapping
- Python binding tree: `finstack-py/src/bindings/{core,analytics,margin,...}/`; `lib.rs` delegates to `bindings::register_root`
- WASM binding tree: `finstack-wasm/src/api/{core_ns,analytics,margin,...}/`; public API via `index.js` facade, not raw pkg/
- Wrapper pattern: `pub(crate) inner: RustType` with `from_inner()` constructor
- Error handling: centralized `core_to_py()` in `errors.rs` (Python), `JsValue::from_str` (WASM); never use `.unwrap()` or `.expect()` in non-test binding code
- Module registration: every submodule sets `__all__` via `PyList` in `register()`; no dynamic export discovery
- Builder pattern: fluent chaining (e.g., `Type.builder(id).field(val).build()`)

## API Conventions

- Accessors use `get_*` naming (e.g., `get_discount()`, `get_forward()`, `get_price()`)
- Metric keys are fully qualified: `bucketed_dv01::USD-OIS::10y`, `cs01::ACME-HZD`, `pv01::usd_ois`
- Z-spread CS01 for bonds uses instrument ID as key (e.g., `cs01::BOND_A`), not `z_spread`
- Bond CS01 without a hazard curve uses z-spread bump method (market convention)

## Naming Strategy

- **Prefer simple, short names across Rust / Python / WASM.** The canonical Rust name should read well as the Python and WASM binding name. If a Rust name is long or awkward (e.g. `period_stats_from_returns`, `rolling_var_forecasts_with_method`), that is a signal the Rust name itself should be shortened, not that the binding should rename it.
- **Triplet consistency is mandatory.** Rust `snake_case` ↔ Python `snake_case` (identical) ↔ WASM `camelCase` (via `#[wasm_bindgen(js_name = ...)]`). `period_stats` / `period_stats` / `periodStats`, not a mix.
- **Short name = canonical / most-common variant.** When multiple variants of one concept exist, give the short name to the variant most binding users will call. Example:
  - `period_stats(returns: &[f64])` — canonical, takes raw flat returns (exposed in Python/WASM)
  - `period_stats_from_grouped(grouped: &[(PeriodId, f64)])` — specialized grouped-input variant (Rust-internal)
  - `rolling_var_forecasts(..., VarMethod)` — canonical, enum-dispatched (exposed)
  - `rolling_var_forecasts_with_fn(..., fn)` — specialized closure variant (Rust-internal)
- **Descriptive suffixes for specialized variants:** use `_from_<input>` (alternate input shape), `_with_<thing>` (alternate dispatch mechanism), `_unchecked` (invariant-skipping). Suffixes are only for the non-canonical variants; the short base name belongs to the one exposed through bindings.
- **Accessors still use `get_*`** (see above) — naming-strategy shortening does not override the `get_*` convention.
- **When renaming, propagate everywhere in one slice:** Rust source + Rust tests + re-exports → PyO3 `#[pyfunction]` + `__all__` + `.pyi` + `__init__.py` → WASM `#[wasm_bindgen(js_name=...)]` + `index.d.ts` + `exports/*.js` → `parity_contract.toml` + benchmarks + notebooks. Verify with `make fmt && make lint && make test && make python-dev`.

## Workflow Preferences

- Preferred flow: Audit/Review → Plan → Implement (in that order)
- When a plan file exists: do NOT edit the plan file; do not recreate todos that already exist; mark todos as `in_progress` when starting each one
- User reports issues by pasting terminal output (clippy, cargo audit, test failures) rather than describing them
- When moving files, use `mv` in terminal and update all import references; then lint and format
