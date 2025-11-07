---
trigger: model_decision
description: This is useful for learning about the scenarios crate and its functionality which includes: Declarative financial statement modeling as directed graphs, period-by-period evaluation with precedence rules (Value > Forecast > Formula), time-series forecasting with deterministic and statistical methods, capital structure integration, dynamic metric registry, and currency-safe arithmetic.
globs:
---
### Finstack Statements (Rust) — Rules, Structure, and Contribution Guide

This document defines Cursor rules for the `finstack/statements/` crate. It explains purpose, structure, invariants, coding standards, and how to add new features safely while preserving determinism, currency safety, and evaluation correctness.

### Scope and Purpose
- **Core responsibilities**: Declarative financial statement modeling as directed graphs, period-by-period evaluation with precedence rules (Value > Forecast > Formula), time-series forecasting with deterministic and statistical methods, capital structure integration, dynamic metric registry, and currency-safe arithmetic.
- **Declarative modeling**: Models are defined as directed acyclic graphs (DAGs) of metrics with dependencies resolved automatically via topological sort.
- **Determinism-first**: Evaluation order is stable; statistical forecasts use explicit seeds for reproducibility; results must be identical across runs.
- **Currency-safety**: Arithmetic on `Money` requires identical currencies; FX conversions are explicit and validated; formulas enforce currency consistency.
- **Serde stability**: Stable field names and enums for `FinancialModelSpec`, `NodeSpec`, `NodeType`, `ForecastMethod`; unknown fields are denied on inbound types.
- **No unsafe code**: The crate uses `#![deny(unsafe_code)]` at the crate level.

### Directory Structure
- `src/lib.rs`: crate facade, module declarations, and public re-exports with prelude.
- `src/types/`: wire types for serialization (`model.rs`, `node.rs`, `value.rs`).
  - `model.rs`: `FinancialModelSpec` and `CapitalStructureSpec`.
  - `node.rs`: `NodeSpec`, `NodeType`, `ForecastSpec`, `ForecastMethod`, `SeasonalMode`.
  - `value.rs`: `AmountOrScalar` (unifies `Money` and `f64` for period values).
- `src/builder/`: type-safe builder API with compile-time state enforcement.
  - `model_builder.rs`: `ModelBuilder<NeedPeriods>` → `ModelBuilder<Ready>` type-state pattern.
  - `mod.rs`: `MixedNodeBuilder` for ergonomic node construction.
- `src/dsl/`: domain-specific language for formulas.
  - `parser.rs`: nom-based parser for formula text.
  - `ast.rs`: `StmtExpr` AST representation.
  - `compiler.rs`: compiles `StmtExpr` → `finstack_core::expr::Expr`.
- `src/evaluator/`: evaluation engine for models.
  - `engine.rs`: `Evaluator` and `EvaluatorWithContext` (main public APIs).
  - `dag.rs`: dependency graph construction and topological sort.
  - `precedence.rs`: precedence resolution (Value > Forecast > Formula) per period.
  - `context.rs`: `EvaluationContext` for variable/function lookups during formula evaluation.
  - `forecast_eval.rs`: applies forecast methods to generate period values.
  - `formula.rs`: formula evaluation with currency validation.
  - `results.rs`: `Results` and `ResultsMeta` types.
- `src/forecast/`: forecast method implementations.
  - `deterministic.rs`: ForwardFill, GrowthPercentage, CurvePercentage.
  - `statistical.rs`: Normal, LogNormal (with explicit seed).
  - `override_method.rs`: Override (sparse period values).
  - `timeseries.rs`: TimeSeries (external data reference).
  - `mod.rs`: Seasonal (patterns with optional growth).
- `src/registry/`: dynamic metric registry for reusable definitions.
  - `schema.rs`: JSON schema for metric definitions.
  - `dynamic.rs`: registry loader and lookup.
  - `builtins.rs`: built-in `fin.*` namespace metrics.
  - `validation.rs`: metric definition validation.
- `src/capital_structure/`: debt/equity instrument integration.
  - `types.rs`: `DebtInstrument` and related types.
  - `integration.rs`: cashflow computation via `finstack-valuations`.
  - `builder.rs`: capital structure builder APIs.
- `src/extensions/`: plugin framework for post-evaluation extensions.
  - `plugin.rs`: `Extension` trait and `ExtensionContext`.
  - `registry.rs`: `ExtensionRegistry` for managing extensions.
  - `corkscrew.rs`: `CorkscrewExtension` for balance sheet roll-forward validation.
  - `scorecards.rs`: `CreditScorecardExtension` for rating assignment.
- `src/results/`: result export utilities.
  - `export.rs`: Polars DataFrame exports (feature-gated under `dataframes`).
- `src/utils/`: internal utilities.
  - `formula.rs`: formula parsing and validation helpers.
- `src/error.rs`: `Error` enum and `Result<T>` type alias.
- `tests/`: integration tests for builder, evaluator, DAG, precedence, forecasts, registry, capital structure, extensions.
- `benches/`: benchmarks for statement evaluation performance.
- `data/`: example model definitions and metric schemas.

### Cross‑Cutting Invariants
- **No `unsafe`**: The crate enforces `#![deny(unsafe_code)]` at the crate level.
- **Period-based evaluation**: All metrics are evaluated over discrete periods (monthly, quarterly, annual); period definitions use `finstack-core::dates::Period`.
- **Precedence rule**: For each metric and period, evaluation follows **Value > Forecast > Formula** priority. Explicit values override forecasts; forecasts override calculated formulas.
- **DAG requirement**: Models must form a directed acyclic graph (DAG); circular dependencies are detected and rejected before evaluation.
- **Determinism**: Topological sort is stable (lexicographic ordering for ties); statistical forecasts use explicit seeds; parallel evaluation (if added) must match serial results exactly.
- **Currency consistency**: Formulas must preserve currency; operations mixing currencies are rejected; `AmountOrScalar` handles `Money` vs scalar transparently.
- **Stable serde**: All wire types (`FinancialModelSpec`, `NodeSpec`, `ForecastSpec`, etc.) maintain stable serialized forms; avoid renaming fields or changing shapes without versioning.
- **Capital structure integration**: Instruments reference `finstack-valuations` types; cashflows are computed with `MarketContext` and aggregated via DSL (`cs.*` namespace).
- **Extension framework**: Extensions run post-evaluation; they can validate, augment, or transform results but must not mutate the model graph.

### Coding Standards (Statements)
- **Naming**: Types are nouns (`FinancialModelSpec`, `NodeSpec`, `Evaluator`); functions are verbs (`evaluate`, `resolve_node_value`, `apply_forecast`). Avoid abbreviations; prefer clarity.
- **Type safety**: Use explicit enums (`NodeType`, `ForecastMethod`) instead of stringly-typed values. Use `AmountOrScalar` to unify `Money` and scalar values.
- **APIs**: Public APIs must be documented with examples where practical. Avoid panics in public paths; return `crate::Result<T>` with `Error` variants.
- **Errors**: Use `Error` enum with variants like `Build`, `FormulaParse`, `Eval`, `NodeNotFound`, `CircularDependency`, `CurrencyMismatch`, `Period`, `Forecast`, `Registry`, `CapitalStructure`. Keep messages actionable with context (node_id, period, formula).
- **Serde**: All wire types serialize cleanly; use `#[serde(rename_all = "snake_case")]` for consistency; add defaults for new fields to preserve backward compatibility.
- **Builder pattern**: `ModelBuilder` uses type-state pattern (`NeedPeriods` → `Ready`) to enforce correct construction order; validates structure before building.
- **DSL design**: Keep parser simple and predictable; support time-series operators (`lag`, `lead`, `diff`, `pct_change`), rolling windows (`rolling_mean`, etc.), statistical functions (`std`, `var`, `median`), and custom functions (`sum`, `mean`, `ttm`, `annualize`, `coalesce`).
- **Evaluation context**: `EvaluationContext` provides variable/function lookups; isolate side effects (market data access, instrument pricing) in context implementations.
- **Performance**: Preallocate vectors where sizes are known; cache compiled expressions in DAG; reuse `EvaluationContext` across periods where safe.
- **Dependencies**: Core dependencies are `finstack-core`, `finstack-valuations`, `indexmap`, `serde`, `serde_json`, `nom` (parser), `thiserror`. Optional: `polars` (for dataframes).
- **Tests**: Add unit tests in module files and integration tests in `tests/` directory. Cover happy paths, edge cases (circular deps, missing nodes, currency mismatches), and golden tests for formula evaluation.

### Feature Design Patterns
- **Type-state builder**: `ModelBuilder<NeedPeriods>` enforces period definition before node addition; transition to `ModelBuilder<Ready>` after `.periods()`. This prevents runtime errors.
- **Precedence resolution**: For each metric and period, check explicit values first, then forecasts, then formulas. This allows users to override forecasts with actuals and override formulas with forecasts.
- **DAG construction**: Build dependency graph from formula references; perform topological sort to determine evaluation order; detect cycles before evaluation.
- **Formula compilation**: Parse formula text → `StmtExpr` AST → compile to `finstack_core::expr::Expr` → evaluate with context. Cache compiled expressions in DAG nodes.
- **Forecast evaluation**: Apply forecast methods to generate period values; methods include deterministic (ForwardFill, Growth, Curve) and statistical (Normal, LogNormal with seed) variants.
- **Currency handling**: `AmountOrScalar` wraps `Money` or `f64`; formulas enforce currency consistency via context; operations mixing incompatible types are rejected.
- **Capital structure integration**: Instruments stored in `CapitalStructureSpec`; accessed via `cs.*` namespace in DSL; cashflows computed with `MarketContext` from valuations crate.
- **Extension plugins**: Extensions implement `Extension` trait; registered in `ExtensionRegistry`; run post-evaluation with access to results and model metadata.
- **Registry system**: Metrics defined in JSON; loaded into dynamic registry; inter-metric dependencies resolved; built-in `fin.*` namespace provided.
- **DataFrame exports**: `Results::to_dataframe()` (feature-gated) converts results to Polars DataFrames for analysis; requires `dataframes` feature.

### Adding New Features to `statements/`

1) New Forecast Method
- Extend `types/node.rs`:
  - Add enum variant to `ForecastMethod` with docs and parameters (e.g., `ExponentialSmoothing { alpha: f64 }`).
  - Update serialization tests to verify serde stability.
- Implement in `forecast/`:
  - Add evaluation logic in appropriate module (deterministic, statistical, or new module).
  - Ensure determinism (use explicit seed for randomness).
  - Return `Vec<AmountOrScalar>` for all periods.
  - Add tests covering edge cases (empty periods, boundary conditions).
- Integration tests in `tests/forecast_methods.rs`.

2) New DSL Function or Operator
- Extend `dsl/ast.rs`:
  - Add variant to `StmtExpr` or function enum.
- Update `dsl/parser.rs`:
  - Add parsing logic using nom combinators.
  - Ensure operator precedence is correct.
- Update `dsl/compiler.rs`:
  - Add compilation to `finstack_core::expr::Expr`.
  - Handle currency propagation and validation.
- Add tests in `tests/dsl_tests.rs` covering syntax, compilation, and evaluation.
- Update documentation in `dsl/mod.rs` with examples.

3) New Extension
- Create module in `extensions/`:
  - Implement `Extension` trait with `name`, `description`, `validate`, and `execute` methods.
  - Access results and model metadata via `ExtensionContext`.
  - Return `ExtensionResult` with status and optional metadata.
- Register in `extensions/registry.rs` if built-in, or provide registration API for user extensions.
- Add tests in `tests/extensions/` covering validation, execution, and edge cases.
- Document usage in module docs with examples.

4) Capital Structure Enhancements
- Extend `capital_structure/types.rs`:
  - Add new instrument types (e.g., `EquityInstrument`, `SwapInstrument`).
  - Ensure types are compatible with `finstack-valuations` instrument traits.
- Update `capital_structure/integration.rs`:
  - Add cashflow computation logic for new instruments.
  - Integrate with `MarketContext` for pricing.
- Extend `cs.*` namespace in DSL to expose new instrument metrics.
- Add tests in `tests/capital_structure_tests.rs`.

5) Registry Enhancements
- Extend `registry/schema.rs`:
  - Add new metric definition fields if needed (e.g., `unit`, `category`).
  - Update JSON schema validation.
- Add built-in metrics in `registry/builtins.rs`:
  - Define metric in `fin.*` namespace with formula and dependencies.
  - Ensure inter-metric dependencies are resolvable.
- Add tests in `tests/registry_tests.rs` covering loading, validation, and dependency resolution.

6) Evaluation Engine Optimizations
- Cache compiled expressions in `evaluator/dag.rs` DAG nodes.
- Implement parallel period evaluation (if safe and deterministic).
- Add benchmarks in `benches/statements_operations.rs` to track performance.
- Ensure serial ≡ parallel results in all tests.

7) Where Clause Extensions
- Extend `NodeSpec` to support conditional logic beyond simple masking.
- Update `evaluator/precedence.rs` to handle complex conditions.
- Add tests for nested conditions, multi-period constraints, and edge cases.

### Review & Testing Checklist
- Public API has doc comments and examples.
- New types implement serde with stable names and defaults.
- No panics in public code paths; return `Result`.
- Determinism maintained (stable topological sort, explicit seeds for randomness).
- Currency safety preserved; formulas enforce currency consistency.
- DAG validation enforced (no circular dependencies).
- Precedence rules respected (Value > Forecast > Formula).
- Builder type-state transitions work correctly.
- Integration tests cover multi-period, multi-metric, and edge cases.
- Capital structure integration tested (if applicable).
- Extension execution tested (if applicable).
- DataFrame exports tested (if `dataframes` feature is enabled).
- Benchmarks updated if performance-critical paths are modified.

### Practical Tips
- Use `ModelBuilder` for all model construction; type-state prevents invalid construction order.
- Always call `.periods()` before adding nodes; builder enforces this at compile-time via type-state.
- For time-series operations in formulas, use DSL functions (`lag`, `lead`, `diff`, `pct_change`, `rolling_mean`) instead of manual indexing.
- When adding forecasts, use explicit seeds for statistical methods to ensure reproducibility.
- For capital structure, ensure instruments are compatible with `finstack-valuations` types and can be serialized/deserialized via serde.
- Use `AmountOrScalar` to unify `Money` and scalar values in node values; formulas handle conversion automatically.
- For extensions, keep logic stateless and deterministic; avoid side effects that would break reproducibility.
- Use `IndexMap` for node storage to ensure stable iteration order (determinism).
- When debugging formula evaluation, check `EvaluationContext` variable bindings and compiled expression structure.
- For complex models, use registry to define reusable metrics rather than duplicating formulas.

### Anti‑Patterns to Avoid
- Constructing models without periods (builder enforces this, but avoid manual construction).
- Implicit currency conversion in formulas (always validate currency consistency).
- Circular dependencies in model graph (DAG validation will reject, but design to avoid).
- Panicking on missing nodes or invalid formulas (return `Result` with context).
- Mutating model after construction (use immutable patterns or rebuild via builder).
- Using raw `String` for node IDs without validation (use `IndexMap` keys for safety).
- Skipping precedence rules (always respect Value > Forecast > Formula order).
- Non-deterministic forecasts without explicit seed (breaks reproducibility).
- Mixing `HashMap` and `IndexMap` (use `IndexMap` for stable iteration).
- Adding extensions with side effects that break determinism.
- Serializing with unstable field names (use `#[serde(rename_all = "snake_case")]`).

### Cargo Features
The `finstack-statements` crate supports the following features:
- `dataframes` (optional): Enables Polars DataFrame exports via `dep:polars`. Provides `Results::to_dataframe()` and related functions.

Default features: `[]` (no defaults; opt-in for dataframes)

### Available Forecast Methods
The `ForecastMethod` enum provides:
- **Deterministic**:
  - `ForwardFill`: Repeats the last known value.
  - `GrowthPercentage { rate }`: Compound growth at specified rate.
  - `CurvePercentage { rates }`: Period-specific growth rates (map of PeriodId → rate).
- **Statistical**:
  - `Normal { mean, std_dev, seed }`: Normal distribution (can be negative).
  - `LogNormal { mean, std_dev, seed }`: Log-normal distribution (always positive).
- **Override**:
  - `Override { base, overrides }`: Base forecast method with sparse period overrides.
- **TimeSeries**:
  - `TimeSeries { source }`: Reference to external data (e.g., from market context or registry).
- **Seasonal**:
  - `Seasonal { pattern, mode, growth_rate }`: Repeating seasonal pattern with optional growth.

All statistical methods require explicit `seed` for reproducibility.

### Available DSL Functions
The formula DSL supports:
- **Arithmetic**: `+`, `-`, `*`, `/`, `^` (exponentiation), unary `-`
- **Comparison**: `<`, `>`, `<=`, `>=`, `==`, `!=`
- **Logical**: `and`, `or`, `not`
- **Time-series**: `lag(expr, n)`, `lead(expr, n)`, `diff(expr, n)`, `pct_change(expr, n)`
- **Rolling windows**: `rolling_mean(expr, n)`, `rolling_sum(expr, n)`, `rolling_std(expr, n)`, `rolling_var(expr, n)`, `rolling_median(expr, n)`, `rolling_min(expr, n)`, `rolling_max(expr, n)`
- **Statistical**: `std(expr)`, `var(expr)`, `median(expr)`, `cumsum(expr)`, `cumprod(expr)`, `cummin(expr)`, `cummax(expr)`
- **Custom**: `sum(expr)`, `mean(expr)`, `ttm(expr)` (trailing twelve months), `annualize(expr, periods_per_year)`, `coalesce(expr1, expr2, ...)` (first non-null)
- **Conditional**: `if(condition, then_expr, else_expr)`
- **Capital structure**: `cs.metric_name` (access capital structure metrics)
- **Registry**: `fin.metric_name` (access built-in metrics from registry)

### Error Types
The `Error` enum provides the following variants:
- `Build`: Model building error (invalid period range, missing periods).
- `FormulaParse`: Formula parsing error (syntax, invalid function).
- `Eval`: Evaluation error (undefined variable, type mismatch).
- `NodeNotFound`: Node not found in model.
- `CircularDependency`: Circular dependency detected (path included in error).
- `CurrencyMismatch`: Currency mismatch in formula (expected vs found).
- `Period`: Period validation error (invalid range, overlapping periods).
- `MissingData`: Missing required data (node value, forecast input).
- `InvalidInput`: Invalid input data (negative periods, invalid parameter).
- `Forecast`: Forecast method error (invalid parameters, execution failure).
- `Registry`: Registry error (metric not found, invalid definition).
- `CapitalStructure`: Capital structure error (invalid instrument, pricing failure).
- `Serde`: Serialization/deserialization error (wrapped `serde_json::Error`).
- `Core`: Core crate error (wrapped `finstack_core::Error`).
- `Io`: I/O error (wrapped `std::io::Error`).
- `BuilderError`: Builder construction error (invalid state transition).
- `IndexError`: Collection access error (invalid index, missing key).

### Future Enhancements
The following features are planned for future releases:
- **Scenario integration**: Apply scenarios to models and compare before/after results with stress testing.
- **Multi-currency models**: Support models with multiple reporting currencies and automatic FX conversion.
- **Optimization solver**: Add optimization extension for calibrating model parameters to target metrics.
- **Statement templates**: Pre-built templates for common financial statements (P&L, balance sheet, cashflow statement).
- **Advanced where clauses**: Support complex conditional logic with nested conditions and multi-period constraints.
- **Parallel evaluation**: Parallelize period evaluation with deterministic aggregation for large models.
- **Enhanced registry**: Support external metric repositories, versioning, and namespacing.
- **Audit trail**: Track evaluation provenance with detailed computation graphs for debugging and compliance.
- **Interactive builder**: Web-based UI for model construction with live validation and preview.

### Testing Guidelines
- **Unit tests**: In module files (types, builder, dsl, evaluator, forecast, registry, extensions).
- **Integration tests**: In `tests/` directory covering:
  - `builder_tests.rs`: Builder validation, type-state transitions, period handling.
  - `evaluator_tests.rs`: DAG construction, topological sort, precedence resolution.
  - `dsl_tests.rs`: Parser, AST, compiler, formula evaluation.
  - `forecast_tests.rs`: All forecast methods with edge cases and determinism checks.
  - `registry_tests.rs`: Metric loading, validation, dependency resolution.
  - `capital_structure_tests.rs`: Instrument integration, cashflow computation.
  - `extensions_tests.rs`: Extension execution, validation, metadata handling.
  - `serde_tests.rs`: Serialization round-trips, backward compatibility.
- **Benchmarks**: In `benches/statements_operations.rs` for performance regression tracking.
- **Golden tests**: Store expected results for complex models to detect behavioral changes.

### Benchmark Guidelines
- Use `criterion` for benchmarks (dev-dependency).
- Benchmark model evaluation with varying sizes (10, 100, 1000 nodes).
- Measure DAG construction, topological sort, formula compilation, and evaluation time.
- Test both simple formulas and complex multi-dependency models.
- Run benchmarks with `cargo bench --package finstack-statements`.
- Results are saved in `target/criterion/` for comparison across runs.

### How to Propose a Change
- Open a small, scoped PR that:
  - Explains the problem and target API.
  - Shows tests that guard behavior and stability.
  - Calls out any serde or compatibility risks explicitly.
  - Demonstrates determinism (stable evaluation order, reproducible results).
  - Follows the coding standards and patterns documented here.
  - Updates benchmarks if performance-critical paths are modified.
  - Documents any new features in docstrings with examples.
  - Updates this guide if new patterns or conventions are introduced.

This file governs only the `statements/` crate. See separate rules for `core/`, `valuations/`, `scenarios/`, `portfolio/`, and Python/WASM bindings.
