**Finstack Repo Overview**
- Deterministic, cross‑platform financial computation engine with Rust core and Python/WASM bindings.
- Emphasizes Decimal numerics, currency safety, strict serde schemas, and predictable performance.
- Workspace version: `0.4.0`; Rust toolchain: `1.90` (workspace).

**Workspace Layout**
- `finstack/` meta‑crate re‑exporting subcrates; examples by domain under `finstack/examples/`.
- `finstack/core` core types, money/FX, time (periods/calendars/day‑count), expression engine, config, errors.
- `finstack/statements` period evaluation with Value > Forecast > Formula; optional Polars DataFrames.
- `finstack/valuations` cashflows, pricing, risk; MC features; TS export support.
- `finstack/portfolio` positions/books, aggregation, scenarios; optional DataFrames.
- `finstack/scenarios` deterministic DSL and execution engine.
- `finstack/io` IO/interop (placeholder crate in this snapshot).
- `finstack-py/` Python bindings (PyO3) and tests; package name `finstack`.
- `finstack-wasm/` WASM bindings (wasm‑bindgen), TS types, examples.
- `packages/finstack-ui/` UI kit (React) that can consume `finstack-wasm`.
- `docs/` technical and UI kit design docs; `tests/` golden values and top‑level resources.
- Top‑level: `Cargo.toml` (workspace), `Makefile`, `README.md`, `CHANGELOG.md`, `MIGRATION_GUIDE.md`, `deny.toml`.

**Crates & Features**
- `finstack-core`
  - Default features: `serde`, `parallel` (Rayon).
  - Provides: `Amount`, `Currency`, `Rate`, FX interfaces, calendars/day‑count, Decimal math, expression DAG.
- `finstack-statements`
  - Optional: `dataframes` (Polars).
- `finstack-valuations`
  - Default: `serde`, `parallel`; Optional: `mc` (Monte Carlo), `strict_validation`, `ts_export`.
- `finstack-portfolio`
  - Default: `scenarios`, `dataframes`, `parallel`.
- `finstack-scenarios` deterministic scenario DSL; depends on core/statements/valuations.
- `finstack-io` reserved for IO formats.
- `finstack` (meta)
  - Feature groups: `core`, `statements`, `valuations`, `scenarios`, `portfolio`, `io`, `all`.
  - Cross‑cutting: `parallel` (maps to core/valuations), `dataframes` (maps to statements/portfolio).

**Development Setup**
- Prereqs: Rust (stable), Node.js (>=18), Python 3.12+ with `uv`.
- Quick start:
  - Rust: `cargo build` or `make build`
  - Run examples: `make examples`
  - Python venv + build: `make python-dev`
  - WASM build: `make wasm-build`

**Build & Test**
- Rust
  - Build: `cargo build` (profiles: `dev`, `release`, `release-perf`, `bench`).
  - Lint: `make lint-rust` (clippy enforced; warnings denied).
  - Tests: `make test-rust` (nextest), `make test-rust-doc` (doctests).
  - Coverage: `make coverage`, `make coverage-html`, `make coverage-lcov`.
- Python (`finstack-py`)
  - Dev env: `make python-dev` (uv + maturin develop).
  - Lint: `make lint-python` / `make lint-python-fix`.
  - Tests: `make test-python` (pytest).
- WASM (`finstack-wasm`)
  - Build: `make wasm-build` (wasm-pack web target).
  - Lint/Format: `make lint-wasm` / `make lint-wasm-fix` / `make fmt-wasm`.
  - Tests: `make test-wasm` (headless browser via wasm-pack).
- UI (`packages/finstack-ui`)
  - Build: `npm run build` in `packages/finstack-ui`.
  - Lint/Format: `make lint-ui` / `make fmt-ui`.
  - Tests: `make test-ui`, coverage: `make test-ui-coverage`.

**Profiles & Performance**
- `release` optimized for speed (LTO thin, symbols stripped).
- `release-size` optimized for size (for WASM bundles).
- `bench` inherits `release` with debug line info for profiling.

**Coding Standards & Lints**
- Clippy policy (workspace): deny panics (`panic`, `unwrap_used`, `expect_used`), indexing without checks, and various correctness/perf issues.
- Numerical: Decimal‑first via `rust_decimal`; avoid lossy float literals.
- Serde stability: strict field names across crates; JSON parity for bindings.
- No `unsafe` patterns; prefer compile‑time validation and explicit FX policies.

**CI**
- GitHub Actions workflow `.github/workflows/build.yml`:
  - Formats (rustfmt), runs clippy across crates, and `cargo test` (release mode) excluding Python crate.
  - Uses stable toolchain with caching and swap setup for linking.

**Policies & Invariants**
- Determinism: Decimal numerics; serial ≡ parallel; stable ordering.
- Currency‑safety: No implicit cross‑currency math; explicit conversions only, policies recorded.
- Time‑series standard: Polars re‑exports for DataFrame/Series surfaces (opt‑in where applicable).
- Policy visibility: Results stamp numeric mode, parallel flag, rounding context, and FX policy.

**Useful Commands**
- Lint everything: `make lint` (Rust, Python, WASM, UI).
- Run full local test suite: `make test` (Rust + doc + Python + WASM + UI).
- Generate docs: `make doc` (rustdoc; workspace crates only, no deps).
- Generate Python stubs: `make stubs`.

**Licensing & Docs**
- Dual‑licensed: `MIT OR Apache-2.0` (see `LICENSE`).
- Additional docs: `README.md`, `CHANGELOG.md`, `MIGRATION_GUIDE.md`, `docs/` (UI kit ADRs/design), `RELEASE_NOTES_0.8.0.md`.

**Getting Help**
- For Python usage, see `finstack-py/README.md`; for WASM usage and examples, see `finstack-wasm/README.md`.

