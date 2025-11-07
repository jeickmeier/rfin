---
trigger: model_decision
description: This is useful for learning about the portfolio crate and its functionality which includes:  Entity-based position tracking, multi-instrument valuation, cross-currency aggregation with explicit FX, attribute-based grouping, metrics aggregation, scenario integration, and DataFrame exports.
globs:
---
### Finstack Portfolio (Rust) — Rules, Structure, and Contribution Guide

This document defines Cursor rules for the `finstack/portfolio/` crate. It explains purpose, structure, invariants, coding standards, and how to add new features safely while preserving determinism, currency safety, and cross-instrument compatibility.

### Scope and Purpose
- **Core responsibilities**: Entity-based position tracking, multi-instrument valuation, cross-currency aggregation with explicit FX, attribute-based grouping, metrics aggregation, scenario integration, and DataFrame exports.
- **Entity-centric model**: Positions belong to entities (companies, funds) with support for standalone instruments via dummy entity (`DUMMY_ENTITY_ID`).
- **Flat position storage**: Simple Vec-based position list with flexible attribute-based grouping (no enforced hierarchy).
- **Multi-instrument support**: Works with any instrument implementing `finstack_valuations::instruments::common::traits::Instrument`.
- **Cross-currency safety**: Explicit FX conversion to portfolio base currency via `FxMatrix`; no implicit currency mixing.
- **Determinism-first**: Results must be identical across serial and parallel execution; aggregation order must not affect outputs.
- **Serde stability**: Stable field names and enums for portfolio and entity types; position serialization is limited due to trait objects.
- **No unsafe code**: The crate uses `#![deny(unsafe_code)]` at the crate level.

### Directory Structure
- `src/lib.rs`: crate facade, module declarations, and public re-exports.
- `src/types.rs`: core types (`Entity`, `EntityId`, `PositionId`, `DUMMY_ENTITY_ID` constant).
- `src/position.rs`: `Position` struct and `PositionUnit` enum (Units, Notional, FaceValue, Percentage).
- `src/portfolio.rs`: `Portfolio` struct with entity registry, position list, validation, and query helpers.
- `src/builder.rs`: `PortfolioBuilder` for ergonomic construction with validation.
- `src/valuation.rs`: `value_portfolio` function, `PortfolioValuation` and `PositionValue` results.
- `src/metrics.rs`: metrics aggregation logic (`aggregate_metrics`, `AggregatedMetric`, `PortfolioMetrics`).
- `src/grouping.rs`: attribute-based grouping (`aggregate_by_attribute`, `group_by_attribute`).
- `src/results.rs`: additional result types and utilities (if needed beyond valuation types).
- `src/scenarios.rs`: scenario application and revaluation (feature-gated, requires `scenarios` feature).
- `src/dataframe.rs`: Polars DataFrame exports for positions and entities (feature-gated, requires `dataframes` feature).
- `src/error.rs`: `PortfolioError` enum and `Result<T>` type alias.
- `tests/`: integration tests covering builder, valuation, FX conversion, grouping, metrics, scenarios.
- `benches/`: benchmarks for portfolio valuation performance.

### Cross‑Cutting Invariants
- **No `unsafe`**: The crate enforces `#![deny(unsafe_code)]` at the crate level.
- **Entity integrity**: All positions must reference valid entities in the portfolio's entity registry; validation enforces this before valuation.
- **Dummy entity**: Standalone instruments (derivatives, FX, deposits without entity ownership) reference `DUMMY_ENTITY_ID`; builder auto-creates this entity if needed.
- **Currency safety**: Position values are converted to portfolio base currency via explicit `FxMatrix` lookups; no implicit cross-currency arithmetic.
- **Determinism**: Aggregation order must not affect results; use `IndexMap` for stable iteration order where needed.
- **Quantity semantics**: Position `quantity` is signed (positive=long, negative=short); `PositionUnit` defines interpretation (Units, Notional, FaceValue, Percentage).
- **Instrument trait**: Positions hold `Arc<dyn Instrument>` allowing any instrument from valuations crate; serialization skips instrument field due to trait object constraints.
- **Stable serde**: Portfolio and Entity types maintain stable serialized forms; avoid renaming fields or changing shapes without explicit versioning.
- **Metrics classification**: Summable metrics (DV01, CS01, Delta, Gamma, Vega, Theta) aggregate across positions; non-summable metrics (YTM, Duration, Spread) are position-specific only.

### Coding Standards (Portfolio)
- **Naming**: Types are nouns (`Portfolio`, `Position`, `Entity`); functions are verbs (`value_portfolio`, `aggregate_by_attribute`). Avoid abbreviations; prefer clarity.
- **Type safety**: Use newtype IDs (`EntityId`, `PositionId`) though currently aliased to `String`; consider migrating to `Id<T>` newtypes from `finstack-core` for stronger type safety.
- **APIs**: Public APIs must be documented with examples where practical. Avoid panics in public paths; return `crate::Result<T>` with `PortfolioError` variants.
- **Errors**: Use `PortfolioError` enum with variants like `UnknownEntity`, `ValuationError`, `FxConversionFailed`, `MissingMarketData`. Keep messages actionable with context (position_id, entity_id).
- **Serde**: Portfolio and Entity serialize cleanly; Position skips instrument field (trait object). Use `#[serde(default, skip_serializing_if = "IndexMap::is_empty")]` for optional fields like tags/meta.
- **Builder pattern**: `PortfolioBuilder` validates as it builds (entity existence, no duplicate IDs); `build()` performs final validation before returning `Portfolio`.
- **Concurrency**: Respect the `parallel` feature flag (inherited from core); do not change outputs when toggled. Valuation can be parallelized per-position in future without changing results.
- **Performance**: Preallocate `IndexMap` where sizes are known; avoid redundant FX lookups (cache rates if needed); use `Arc` to share instrument references across positions.
- **No implicit FX**: Never auto-convert `Money` across currencies; require explicit `FxMatrix` and `FxQuery` with policy stamping where appropriate.
- **Dependencies**: Core dependencies are `finstack-core`, `finstack-valuations`, `indexmap`, `serde`, `thiserror`, `time`, `tracing`. Optional: `finstack-scenarios`, `finstack-statements`, `polars` (for dataframes).
- **Tests**: Add unit tests in module files and integration tests in `tests/` directory. Ensure cross-currency, multi-entity, and edge cases are covered. Validate FX fallback behavior.

### Feature Design Patterns
- **Entity-based organization**: Entities own positions; positions reference entities by ID. Use `DUMMY_ENTITY_ID` for standalone instruments (derivatives, FX).
- **Flat position storage**: Positions stored in `Vec<Position>`; no enforced hierarchy. Grouping and aggregation are query-time operations via `group_by_attribute` and `aggregate_by_attribute`.
- **Valuation flow**: (1) Iterate positions, (2) Price each instrument with metrics via `instrument.price_with_metrics()`, (3) Scale by quantity, (4) Convert to base currency via `FxMatrix`, (5) Aggregate by entity.
- **Metrics handling**: Standard summable metrics (DV01, CS01, Theta) are aggregated; non-summable metrics (YTM, Duration) are position-specific. Use `aggregate_metrics` to sum compatible metrics across positions.
- **Attribute-based grouping**: Positions have `tags: IndexMap<String, String>` for flexible grouping (rating, sector, asset_class, etc.). Functions `group_by_attribute` and `aggregate_by_attribute` enable rollups by any tag.
- **Scenario integration**: `apply_scenario` (feature-gated) applies scenario to market data and re-values portfolio; `apply_and_revalue` is a convenience wrapper. Requires `scenarios` feature.
- **DataFrame exports**: `positions_to_dataframe` and `entities_to_dataframe` (feature-gated) convert results to Polars DataFrames for analysis. Requires `dataframes` feature.
- **Builder ergonomics**: `PortfolioBuilder` uses method chaining (`.entity()`, `.position()`, `.base_ccy()`, `.as_of()`) and auto-creates dummy entity if positions reference it. Final `build()` validates entity integrity.

### Adding New Features to `portfolio/`

1) New Position Unit Type
- Extend `position.rs`:
  - Add enum variant to `PositionUnit` with docs (e.g., `Notional(Option<Currency>)`, `FaceValue`, `Percentage`).
  - Update serialization tests to verify serde stability (use `#[serde(rename_all = "snake_case")]`).
  - Consider if unit affects valuation logic in `valuation.rs` (currently quantity is multiplied directly; some units may need special handling).
  - Add tests for new unit type in `tests/portfolio_and_builder.rs`.

2) New Aggregation or Grouping Function
- Add to `grouping.rs`:
  - Implement new aggregation function (e.g., `aggregate_by_entity_and_tag`, `group_by_nested_attribute`).
  - Use `IndexMap` for stable iteration order; ensure deterministic results.
  - Return `Result<IndexMap<K, V>>` for consistency; use `PortfolioError::IndexError` for missing keys.
  - Add tests covering single-group, multi-group, empty portfolio, and missing tag cases.
  - Integration tests in `tests/grouping_and_df.rs`.

3) New Metrics Aggregation
- Extend `metrics.rs`:
  - Add new summable metrics to `standard_portfolio_metrics()` if widely supported.
  - Implement aggregation logic in `aggregate_metrics` for new metric types (e.g., bucketed DV01, CS01 series).
  - Handle non-summable metrics by storing per-position only (do not aggregate).
  - Add tests for new metrics in `tests/metrics_agg.rs` with multi-position portfolios.
  - Ensure metrics are scaled by position quantity before aggregation.

4) Enhanced FX Handling
- Modify `valuation.rs`:
  - Add FX policy stamping to `PositionValue` (e.g., `fx_policy: Option<FxPolicyMeta>`) to audit conversion strategy.
  - Support bucketed FX conversions (e.g., per-tenor for swap cashflows).
  - Add FX caching at portfolio level (bounded LRU) to avoid redundant lookups for same currency pairs.
  - Ensure conversion failures return `PortfolioError::FxConversionFailed` with full context.
  - Add tests for missing FX rates, triangulation, and policy stamping in `tests/valuation_fx.rs`.

5) Scenario Application Enhancements
- Extend `scenarios.rs` (requires `scenarios` feature):
  - Add multi-scenario valuation (e.g., `apply_scenarios` for batch processing).
  - Support scenario composition and chaining.
  - Cache base market state to avoid rebuilding for each scenario.
  - Add before/after comparison helpers (delta value, delta metrics).
  - Integration tests in `tests/scenarios_integration.rs` covering stress tests, parallel shifts, and composition.

6) DataFrame Export Extensions
- Extend `dataframe.rs` (requires `dataframes` feature):
  - Add metric exports to DataFrames (e.g., `metrics_to_dataframe`).
  - Support hierarchical aggregations (entity → position → metrics).
  - Add schema validation and type safety for DataFrame columns.
  - Ensure stable column ordering and naming for downstream consumers.
  - Add tests for empty portfolios, missing metrics, and schema consistency in `tests/grouping_and_df.rs`.

7) Book Hierarchy (Future Enhancement)
- Add `src/book.rs`:
  - Define `Book` struct with nested hierarchy (folder-like structure).
  - Extend `Portfolio` with `books: IndexMap<BookId, Book>`.
  - Add builder methods for book creation and position assignment.
  - Implement aggregation by book level with rollup to parent books.
  - Maintain backward compatibility (flat position list is default; books are optional).

### Review & Testing Checklist
- Public API has doc comments and examples.
- New types implement serde (where feasible) with stable names and defaults.
- No panics in public code paths; return `Result`.
- Determinism maintained (aggregation order independent).
- Currency safety preserved; all FX conversions are explicit via `FxMatrix`.
- Entity integrity validated (all position references are valid).
- Builder validation enforced (duplicate checks, entity existence).
- Integration tests cover multi-entity, cross-currency, and edge cases.
- Scenarios integration tested (if `scenarios` feature is enabled).
- DataFrame exports tested (if `dataframes` feature is enabled).
- Benchmarks updated if performance-critical paths are modified.

### Practical Tips
- Use `PortfolioBuilder` for all portfolio construction; avoid direct `Portfolio::new()` unless you have a specific reason.
- Always call `portfolio.validate()` before valuation if constructing manually; builder does this automatically.
- For standalone instruments, use `DUMMY_ENTITY_ID` as entity_id; builder will auto-create this entity.
- Position tags are flexible; establish conventions for your use case (e.g., `"asset_class"`, `"rating"`, `"sector"`, `"strategy"`).
- When adding metrics, classify as summable or non-summable; only aggregate summable metrics across positions.
- For cross-currency portfolios, always provide `FxMatrix` in `MarketContext`; valuation will fail if FX rates are missing.
- Use `IndexMap` instead of `HashMap` for entity and position collections to ensure stable iteration order (determinism).
- Instrument references are `Arc<dyn Instrument>`; clone `Arc` to share instruments across multiple positions efficiently.
- For large portfolios, consider parallel valuation (future enhancement); ensure results match serial valuation exactly.

### Anti‑Patterns to Avoid
- Constructing portfolios without validation (always use `PortfolioBuilder` or call `validate()` explicitly).
- Implicit cross-currency conversion (always use explicit `FxMatrix` lookups with error handling).
- Using `HashMap` instead of `IndexMap` for entities or positions (breaks determinism).
- Aggregating non-summable metrics (YTM, Duration, Spread) across positions (invalid operation).
- Panicking on missing FX rates or valuation failures (return `Result` with context).
- Mutating positions after adding to portfolio (use immutable patterns or rebuild via builder).
- Using raw `String` for entity_id and position_id without validation (consider migrating to `Id<T>` newtypes).
- Skipping builder validation for "performance reasons" (validation is cheap and prevents runtime errors).
- Assuming all instruments return full metrics (use fallback to `value()` if `price_with_metrics()` fails).
- Serializing `Position` with instrument field (not supported due to trait object; store instrument_id and reconstruct).

### Cargo Features
The `finstack-portfolio` crate supports the following features:
- `scenarios` (default): Enables scenario application and revaluation via `finstack-scenarios` and `finstack-statements` dependencies. Provides `apply_scenario` and `apply_and_revalue` functions.
- `dataframes` (default): Enables Polars DataFrame exports via `dep:polars`. Provides `positions_to_dataframe` and `entities_to_dataframe` functions.

Default features: `["scenarios", "dataframes"]`

### Available Types and Functions

#### Core Types
- `Portfolio`: Main portfolio struct with entity registry and position list.
- `Entity`: Entity representation with ID, name, tags, and metadata.
- `Position`: Position in an instrument with quantity, unit, tags, and metadata.
- `PositionUnit`: Enum for quantity interpretation (Units, Notional, FaceValue, Percentage).
- `PortfolioBuilder`: Builder for ergonomic portfolio construction with validation.

#### Valuation
- `value_portfolio(portfolio, market, config) -> Result<PortfolioValuation>`: Values all positions with cross-currency aggregation.
- `PortfolioValuation`: Results with per-position values, entity aggregates, and total.
- `PositionValue`: Single position valuation with native and base currency values.

#### Grouping and Aggregation
- `aggregate_by_attribute(valuation, positions, tag_key, base_ccy) -> Result<IndexMap<String, Money>>`: Aggregate values by tag.
- `group_by_attribute(positions, tag_key) -> IndexMap<String, Vec<&Position>>`: Group positions by tag.

#### Metrics
- `aggregate_metrics(valuation, base_ccy, fx_matrix, as_of) -> Result<PortfolioMetrics>`: Aggregate summable metrics.
- `PortfolioMetrics`: Portfolio-level aggregated metrics (DV01, CS01, Delta, Gamma, Vega, Theta).
- `AggregatedMetric`: Single aggregated metric with value and currency.

#### Scenarios (feature-gated)
- `apply_scenario(portfolio, scenario, market, config) -> Result<(MarketContext, ScenarioReport)>`: Apply scenario to market data.
- `apply_and_revalue(portfolio, scenario, market, config) -> Result<(PortfolioValuation, ScenarioReport)>`: Apply scenario and revalue portfolio.

#### DataFrames (feature-gated)
- `positions_to_dataframe(valuation) -> Result<DataFrame>`: Export position-level results to Polars DataFrame.
- `entities_to_dataframe(valuation) -> Result<DataFrame>`: Export entity-level aggregates to Polars DataFrame.

### Error Types
The `PortfolioError` enum provides the following variants:
- `UnknownEntity`: Position references an entity not in the portfolio.
- `ValidationFailed`: Portfolio structure validation failed.
- `FxConversionFailed`: Cross-currency conversion failed (missing rate).
- `ValuationError`: Instrument pricing failed for a position.
- `ScenarioError`: Scenario application failed (feature-gated).
- `MissingMarketData`: Required market data (curves, FX) not available.
- `Core`: Wrapped `finstack_core::Error`.
- `InvalidInput`: Invalid input data.
- `BuilderError`: Builder construction failed.
- `IndexError`: Collection access error.

### Future Enhancements
The following features are planned for future releases:
- **Full metrics computation**: Integrate all metrics from `price_with_metrics` with proper bucketed metric aggregation (DV01 by tenor, CS01 by name).
- **Statement aggregation**: Attach financial models to entities and aggregate statements (P&L, balance sheet) to portfolio level.
- **Book hierarchy**: Optional nested book/folder structure for organization with multi-level rollups.
- **Performance optimization**: Parallel per-position valuation with deterministic aggregation; bounded FX rate caching.
- **Position serialization**: Store instrument state alongside position for full serialization/deserialization.
- **Stronger ID types**: Migrate `EntityId` and `PositionId` from `String` aliases to `Id<Entity>` and `Id<Position>` newtypes for type safety.
- **Time-series valuations**: Value portfolio across multiple dates with period alignment and time-series DataFrame exports.

### Testing Guidelines
- **Unit tests**: In module files (types, position, portfolio, error, grouping, metrics).
- **Integration tests**: In `tests/` directory covering:
  - `portfolio_and_builder.rs`: Builder validation, entity integrity, position queries.
  - `valuation_fx.rs`: Cross-currency valuation, FX fallback, rate lookup failures.
  - `valuation_fallback.rs`: Metrics fallback to value-only when `price_with_metrics` fails.
  - `grouping_and_df.rs`: Attribute-based grouping, DataFrame exports, schema validation.
  - `metrics_agg.rs`: Metrics aggregation, summable vs non-summable, scaling by quantity.
  - `scenarios_integration.rs`: Scenario application, revaluation, before/after comparison.
- **Benchmarks**: In `benches/portfolio_valuation.rs` for performance regression tracking.
- **Test market data**: Use `common/mod.rs` helper to build reusable market contexts (discount curves, FX matrix).

### Benchmark Guidelines
- Use `criterion` for benchmarks (dev-dependency).
- Benchmark `value_portfolio` with varying portfolio sizes (10, 100, 1000 positions).
- Measure per-position valuation time, FX lookup overhead, and aggregation cost.
- Test both single-currency and cross-currency portfolios.
- Run benchmarks with `cargo bench --package finstack-portfolio`.
- Results are saved in `target/criterion/` for comparison across runs.

### How to Propose a Change
- Open a small, scoped PR that:
  - Explains the problem and target API.
  - Shows tests that guard behavior and stability.
  - Calls out any serde or compatibility risks explicitly.
  - Demonstrates determinism (aggregation order independence).
  - Follows the coding standards and patterns documented here.
  - Updates benchmarks if performance-critical paths are modified.
  - Documents any new features in docstrings with examples.

This file governs only the `portfolio/` crate. See separate rules for `core/`, `valuations/`, `scenarios/`, `statements/`, and Python/WASM bindings.

