---
name: scenarios-market-standards-upgrade
overview: Bring the finstack_scenarios crate to market-standard quality across conventions, math, numerical stability, performance, safety, and API/design, with a focus on deterministic, well-documented scenario behavior.
todos:
  - id: wire-rate-binding-spec
    content: Wire full RateBindingSpec into the engine and implement tenor/compounding-aware rate extraction in statements adapter.
    status: in_progress
  - id: time-roll-conventions
    content: Make time roll-forward calendar-aware and configurable, using ExecutionContext::calendar and Tenor-based arithmetic.
    status: pending
  - id: cds-hazard-dual-mode
    content: Refactor ParCDS hazard shocks into approximate and calibrated modes with diagnostics and configuration.
    status: pending
  - id: vol-arbitrage-and-kind
    content: Integrate VolSurfaceKind routing and invoke vol-surface arbitrage checks after shocks with configurable strictness.
    status: pending
  - id: instrument-attr-shocks
    content: Implement attribute-based instrument shocks with clear matching semantics and tests.
    status: pending
  - id: structured-credit-engine-support
    content: Wire correlation-related OperationSpec variants into engine support for StructuredCredit instruments with clamping and tests.
    status: pending
  - id: rounding-context-metadata
    content: Connect ApplicationReport.rounding_context and related metadata to the real numeric/rounding context.
    status: pending
  - id: batch-bumps-and-bench
    content: Batch market bump application in the engine and add benchmarks for curve/vol shocks and full scenarios.
    status: pending
  - id: e2e-tests-orchestration
    content: Add end-to-end engine/orchestration tests covering multi-operation scenarios and error/warning propagation.
    status: pending
  - id: docs-and-bindings-update
    content: Align Rust docs, deprecations, and Python/WASM bindings with the new behavior and capabilities.
    status: pending
---

## Plan: Market-Standards Upgrade for `finstack_scenarios`

### 1. Conventions & Core Wiring

- **1.1 Rate binding overhaul**
- Introduce a new engine-facing binding structure that carries full `RateBindingSpec` information instead of a bare `IndexMap<String, String>` map (e.g., `IndexMap<String, RateBindingSpec>`), updating references in [`finstack/scenarios/src/engine.rs`](finstack/scenarios/src/engine.rs).
- Implement `update_rate_from_binding(binding: &RateBindingSpec, model: &mut FinancialModelSpec, market: &MarketContext) -> Result<()>` in [`finstack/scenarios/src/adapters/statements.rs`](finstack/scenarios/src/adapters/statements.rs) that:
- Extracts a rate at `binding.tenor` using the underlying curve’s conventions.
- Converts from curve quoting to the requested `Compounding` and optional `day_count` override.
- Provides clear errors when tenor is out of range or conventions are incompatible.
- Wire engine Phase 2 to use `RateBindingSpec` (and keep the legacy `(node_id, curve_id)` path as a thin adapter or deprecate via feature flag).
- **1.2 Time roll conventions**
- Refactor `apply_time_roll_forward` in [`finstack/scenarios/src/adapters/time_roll.rs`](finstack/scenarios/src/adapters/time_roll.rs) to:
- Use `Tenor::parse(period_str)` plus calendar-aware date arithmetic (via `finstack_core::dates`) instead of `parse_period_to_days` for the main path.
- Respect `ExecutionContext::calendar` when present, including business-day convention and end-of-month rules.
- Keep `parse_period_to_days` for explicitly approximate modes, clearly documented as such.
- Add configuration (e.g., enum or flags) to select between calendar-day and business-day roll modes, with a sensible default.
- **1.3 Tenor/year-fraction alignment**
- Generalize `parse_tenor_to_years_with_context` in [`finstack/scenarios/src/utils.rs`](finstack/scenarios/src/utils.rs) to accept `DayCount` and `BusinessDayConvention` parameters, and:
- Provide small wrapper helpers that derive those from curves (discount/forward) or from `RateBindingSpec`.
- Replace hard-coded Act/Act + ModifiedFollowing usages where alignment with curve conventions is required.

### 2. CDS Hazard Shocks: Dual-Mode Calibration

- **2.1 Hazard bump strategy abstraction**
- Define a small internal abstraction (e.g., `enum HazardBumpMode { Approximate, Calibrated }`) in [`finstack/scenarios/src/adapters/curves.rs`](finstack/scenarios/src/adapters/curves.rs) or a dedicated module.
- Extract the current λ ≈ spread/(1−R) logic into an `Approximate` implementation, making its limitations explicit in comments and error messages.
- **2.2 Calibrated hazard bump mode**
- Implement a `Calibrated` hazard bump that:
- Uses existing credit pricing primitives in `finstack_core`/`finstack_valuations` (or a math/calibration helper if available) to re-solve for hazard knots so that par-spread PVs match shocked spreads.
- Respects existing day-count, payment tenor, and curve conventions for discounting and fee legs.
- Provide a configuration path (feature flag or runtime setting) to select `Approximate` vs `Calibrated` default behavior, with `Calibrated` as the recommended production mode.
- **2.3 Diagnostics and safety checks**
- Add diagnostics to scenario results (e.g., warnings or a debug structure) indicating which hazard bump mode was used and where large differences between approximate and calibrated modes occur.
- Implement guards for extreme inputs (very high spreads, recovery near 1) that emit warnings or force calibrated mode where approximation error would be large.

### 3. Volatility Surfaces & Arbitrage Enforcement

- **3.1 Wire VolSurfaceKind into routing**
- Update `VolAdapter` in [`finstack/scenarios/src/adapters/vol.rs`](finstack/scenarios/src/adapters/vol.rs) to:
- Use `VolSurfaceKind` to select the correct collection in `MarketContext` (equity/FX/IR, etc.) or to annotate `MarketBump` so lower layers can route correctly.
- Validate that a surface with the given ID exists for the specified kind before constructing a bump.
- **3.2 Invoke arbitrage checks post-shock**
- Extend the vol adapter or underlying surface builders to:
- Retrieve the shocked surface grid (expiries, strikes, vols) and call `check_arbitrage` after applying `VolSurfaceParallelPct` and `VolSurfaceBucketPct`.
- Convert any `ArbitrageViolation` into warnings by default, with an option to treat them as hard errors.
- Document the arbitrage checks and configuration knobs clearly in the module docs and crate-level docs.
- **3.3 Tests and tolerances**
- Add unit tests and small integration tests that:
- Verify arbitrage-free surfaces remain violation-free after typical shocks.
- Confirm that intentionally arbitrage-violating shocks are detected and surfaced.

### 4. Instrument Shocks by Attributes (Full Implementation)

- **4.1 Attribute matching semantics**
- Specify and document matching behavior for `InstrumentPricePctByAttr` and `InstrumentSpreadBpByAttr` in [`finstack/scenarios/src/spec.rs`](finstack/scenarios/src/spec.rs):
- Attribute keys/values case sensitivity, AND vs OR semantics, and treatment of missing attributes.
- **4.2 Implement attribute-based filtering**
- Implement `apply_instrument_attr_price_shock` and `apply_instrument_attr_spread_shock` in [`finstack/scenarios/src/adapters/instruments.rs`](finstack/scenarios/src/adapters/instruments.rs) to:
- Iterate over `&mut [Box<dyn Instrument>] `and use instrument attributes metadata to select matches based on the specified `IndexMap<String, String>`.
- Apply shocks via `scenario_overrides_mut` when available, falling back to metadata tags (consistent with type-based shocks).
- Simplify the engine’s attr-handling path in [`finstack/scenarios/src/engine.rs`](finstack/scenarios/src/engine.rs) by removing the `empty_instruments` fallback and treating missing instruments as a clear error or warning.
- **4.3 Tests and consistency**
- Add targeted tests where a mixed portfolio is shocked by sector/rating/region attributes and verify only intended instruments are affected.
- Ensure attr-based and type-based shocks interplay cleanly (e.g., order of application, conflict resolution rules).

### 5. Structured-Credit Correlation Operations via Engine

- **5.1 Clarify engine support model**
- Decide on an explicit model for handling `AssetCorrelationPts`, `PrepayDefaultCorrelationPts`, `RecoveryCorrelationPts`, and `PrepayFactorLoadingPts` in the engine:
- Either require a typed `StructuredCredit` collection in `ExecutionContext` (e.g., a separate slice) or introduce a trait-based adapter hook that can downcast or delegate to structured-credit-aware code.
- **5.2 Wire operations to existing utilities**
- In [`finstack/scenarios/src/adapters/asset_corr.rs`](finstack/scenarios/src/adapters/asset_corr.rs) and engine Phase 1:
- Replace the current warning-only implementation with logic that:
- Locates structured-credit instruments.
- Applies `apply_asset_correlation_shock`, `apply_prepay_default_correlation_shock`, or `apply_selective_correlation_shock` as appropriate.
- Preserve clamping behavior and warnings when requested shocks exceed valid ranges.
- **5.3 Tests and diagnostics**
- Add unit/integration tests that:
- Construct small structured-credit examples and verify correlation parameters change as expected and remain in valid ranges.
- Confirm that warnings are emitted when clamping occurs.

### 6. Rounding, Numeric Context, and Metadata

- **6.1 Rounding context integration**
- Add a way for `ExecutionContext` or the engine to access the active `RoundingContext`/numeric mode from `finstack_core`.
- Update `ApplicationReport` in [`finstack/scenarios/src/engine.rs`](finstack/scenarios/src/engine.rs) to store a stable identifier (e.g., context name or hash) instead of the hard-coded "default".
- **6.2 Extended result metadata (optional)**
- Optionally extend `ApplicationReport` or a sibling diagnostics struct with flags for numeric mode (Decimal vs float), parallelism, and hazard-bump/vol-arb enforcement modes.
- Ensure any additions are serde-stable and documented.

### 7. Performance Improvements (Batched Bumps & Hot Paths)

- **7.1 Batch MarketBump application**
- Refactor engine Phase 1 in [`finstack/scenarios/src/engine.rs`](finstack/scenarios/src/engine.rs) to:
- Collect all `ScenarioEffect::MarketBump` instances into a vector per phase.
- Apply them in bulk via a single `ctx.market.apply_bumps(&bumps)` call per phase, while preserving deterministic ordering.
- **7.2 Curve rebuild efficiency**
- Review and, where possible, reduce allocations in curve rebuild paths in [`finstack/scenarios/src/adapters/curves.rs`](finstack/scenarios/src/adapters/curves.rs) (e.g., reuse buffers, avoid repeated cloning of vectors when applying multiple node shocks).
- **7.3 Benchmarks**
- Add benchmark suites (Rust benches or criterion-based) for:
- Curve node shocks (discount/forward/hazard/inflation) over large grids.
- Vol surface shocks (parallel and bucket).
- Full engine runs over portfolios with 100, 1,000, and 10,000 instruments and varying scenario sizes.

### 8. Engine & Orchestration Tests

- **8.1 End-to-end scenario tests**
- Create integration tests in `finstack/scenarios` that:
- Build small but realistic `MarketContext`, `FinancialModelSpec`, and instrument sets.
- Apply scenarios combining FX, curves, vol, statements, instruments, time-roll, and rate bindings.
- Assert on both `ApplicationReport` and key observable quantities (e.g., PV, statement node values).
- **8.2 Error and warning propagation**
- Add tests that validate how errors and warnings are propagated when:
- Market data is missing or tenors are not found.
- Unsupported operations are specified (where still allowed) or when invalid parameter values (NaN, inf) occur.

### 9. Documentation & API Clarity

- **9.1 Align docs with behavior**
- Review crate-level docs in [`finstack/scenarios/src/lib.rs`](finstack/scenarios/src/lib.rs) and module docs (`adapters/*`, `spec.rs`, `utils.rs`) to:
- Explicitly describe approximations (time-roll, hazard approximation in Approximate mode) and validation steps (vol arbitrage, correlation clamping).
- Clarify which operations are supported in all environments and which require certain features or instrument types.
- **9.2 Public API stability & deprecations**
- If any legacy paths are to be phased out (e.g., raw `(node_id, curve_id)` bindings), mark them with Rust deprecation attributes and explain migration paths to the new `RateBindingSpec`-based APIs.
- **9.3 Python/WASM parity (if in scope)**
- Ensure Python bindings in [`finstack-py/src/scenarios/spec.rs`](finstack-py/src/scenarios/spec.rs) and any WASM bindings reflect the updated specs (e.g., RateBindingSpec usage, new metadata fields) and update docs such as [`finstack-py/docs/SCENARIOS_BINDINGS.md`](finstack-py/docs/SCENARIOS_BINDINGS.md) accordingly.

### 10. Validation & Sign-Off

- **10.1 Golden-copy comparison**
- Establish a small suite of golden scenarios (JSON/YAML) and corresponding expected outputs (PV changes, statement adjustments, warnings) to be used as regression tests before and after changes.
- **10.2 Internal review and calibration checks**
- Have quant reviewers validate CDS hazard and vol arbitrage behavior against independent calculators for a curated set of scenarios.
