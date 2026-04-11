# Full Binding Coverage Plan — Match Rust 100%

**Date**: 2026-04-11
**Scope**: Python + WASM bindings for all 10 domains
**Prerequisite**: Greenfield rewrite stages 0–5f complete (skeleton + core + correlation done)

## Principles

- Rust `pub use` at crate root = binding surface. Every re-exported type/fn gets a binding.
- Wrapper pattern: `Py*` / WASM class wraps `pub(crate) inner: RustType` with `from_inner()`.
- Traits → spec+build pattern (user builds from spec, gets trait-object wrapper).
- No logic in bindings — only type conversion, construction, error mapping.
- Each phase: Python .rs → WASM .rs → `__init__.py` → `.pyi` stubs → WASM exports/*.js → `index.d.ts` → tests → clippy verify.
- `_with_scratch` variants: skip in bindings (Rust optimization detail, not user-facing).

## Current Coverage Summary

| Domain | Status |
|--------|--------|
| core | Complete |
| correlation | ~95% (minor gaps) |
| analytics | ~15% (Performance + 11 fns) |
| monte_carlo | ~5% (European GBM only) |
| margin | ~10% (VM + CSA + enums) |
| statements | ~10% (spec enums + JSON) |
| valuations | ~5% (JSON validation only) |
| statements_analytics | ~10% (4 JSON helpers) |
| portfolio | ~10% (JSON helpers) |
| scenarios | ~15% (spec/template helpers) |

---

## Phase 1: Self-Contained Domains (no cross-crate deps beyond core)

### 1A — Correlation Completion

**Gap**: Missing concrete copula structs (`GaussianCopula`, `StudentTCopula`, `MultiFactorCopula`, `RandomFactorLoadingCopula`), concrete recovery types (`ConstantRecovery`, `CorrelatedRecovery`), `Error` enum.

**Work**:
- [ ] Python: Add `GaussianCopula`, `StudentTCopula`, `MultiFactorCopula`, `RandomFactorLoadingCopula` as optional direct constructors (users can also use `CopulaSpec.build()`)
- [ ] Python: Add `ConstantRecovery`, `CorrelatedRecovery` as optional direct constructors
- [ ] Python: Map `finstack_correlation::Error` variants to a `CorrelationError` Python exception
- [ ] WASM: Add factor model types (`FactorSpec`, `FactorModel`, `SingleFactorModel`, `TwoFactorModel`, `MultiFactorModel`)
- [ ] WASM: Add `CorrelatedBernoulli`, `cholesky_decompose`
- [ ] WASM: Add `CopulaSpec` missing flags (`isRfl`, `isMultiFactor`)
- [ ] WASM: Add `RecoverySpec.marketStandardStochastic()`
- [ ] Update `.pyi`, `__init__.py`, `exports/correlation.js`, `index.d.ts`
- [ ] Tests for new types

### 1B — Analytics Full Coverage

**Gap**: ~80 missing functions + ~15 missing types out of 95 crate-root exports.

**Python work** (split into subfiles under `bindings/analytics/`):

- [ ] `bindings/analytics/mod.rs` → keep `Performance` class, add missing methods:
  - `estimate_ruin`, `rolling_volatility`, `rolling_sortino`, `rolling_sharpe`
  - `multi_factor_greeks`, `drawdown_details`, `beta`, `greeks`, `rolling_greeks`
  - `lookback_returns`, `period_stats`, `excess_returns`
  - `cumulative_returns_outperformance`, `drawdown_outperformance`, `stats_during_bench_drawdowns`
  - `benchmark_idx`, `freq`, `uses_log_returns` getters
- [ ] `bindings/analytics/types.rs` → wrap all result structs:
  - `PeriodStats`, `BetaResult`, `GreeksResult`, `RollingGreeks`, `MultiFactorResult`
  - `DrawdownEpisode`, `LookbackReturns`
  - `RollingSharpe`, `RollingSortino`, `RollingVolatility`
  - `RuinDefinition` (enum), `RuinModel`, `RuinEstimate`
  - `BenchmarkAlignmentPolicy` (enum)
- [ ] `bindings/analytics/functions.rs` → wrap all standalone functions:
  - **aggregation**: `group_by_period`, `period_stats`
  - **benchmark**: `align_benchmark`, `align_benchmark_with_policy`, `calc_beta`, `greeks`, `rolling_greeks`, `tracking_error`, `information_ratio`, `r_squared`, `up_capture`, `down_capture`, `capture_ratio`, `batting_average`, `multi_factor_greeks`, `treynor`, `m_squared`, `m_squared_from_returns`
  - **consecutive**: `count_consecutive`
  - **drawdown**: `avg_drawdown`, `average_drawdown`, `drawdown_details`, `max_drawdown` (from levels), `max_drawdown_duration`, `cdar`, `ulcer_index`, `pain_index`, `calmar`, `calmar_from_returns`, `martin_ratio`, `martin_ratio_from_returns`, `recovery_factor`, `recovery_factor_from_returns`, `sterling_ratio`, `sterling_ratio_from_returns`, `burke_ratio`, `pain_ratio`, `pain_ratio_from_returns`
  - **lookback**: `mtd_select`, `qtd_select`, `ytd_select`, `fytd_select`
  - **returns**: `clean_returns`, `excess_returns`, `convert_to_prices`, `rebase`
  - **risk_metrics return_based**: `cagr`, `cagr_from_periods`, `downside_deviation`, `geometric_mean`, `omega_ratio`, `gain_to_pain`, `modified_sharpe`, `estimate_ruin`
  - **risk_metrics rolling**: `rolling_sharpe`, `rolling_sortino`, `rolling_volatility`, `rolling_sharpe_values`, `rolling_sortino_values`, `rolling_volatility_values`
  - **risk_metrics tail**: `cornish_fisher_var`, `parametric_var`, `skewness`, `kurtosis`, `tail_ratio`, `outlier_win_ratio`, `outlier_loss_ratio`
  - Expand existing fns (`mean_return`, `sharpe`, `sortino`, `volatility`, `value_at_risk`, `expected_shortfall`) with full Rust signature parity (annualize flag, ann_factor)

**WASM work**: Mirror all the above as `#[wasm_bindgen]` functions/classes.

**Artifacts**: `.pyi`, `__init__.py`, `exports/analytics.js`, `index.d.ts`, tests.

### 1C — Monte Carlo Full Coverage

**Gap**: Only European GBM exposed; full engine, processes, payoffs, pricers missing.

**Python work** (split into subfiles under `bindings/monte_carlo/`):

- [ ] `bindings/monte_carlo/mod.rs` → orchestration + `register()`
- [ ] `bindings/monte_carlo/engine.rs` → wrap:
  - `McEngine` (via builder pattern), `McEngineBuilder`, `McEngineConfig`
  - `PathCaptureConfig`, `PathCaptureMode` (enum)
  - `TimeGrid`
  - `MonteCarloResult` (full, with optional paths), `MoneyEstimate`
- [ ] `bindings/monte_carlo/rng.rs` → wrap:
  - `PhiloxRng`, `SobolRng` (behind `mc` feature)
- [ ] `bindings/monte_carlo/process.rs` → wrap all processes:
  - `GbmProcess`, `GbmParams`, `MultiGbmProcess`
  - `BrownianProcess`, `BrownianParams`, `MultiBrownianProcess`
  - `MultiOuProcess`, `MultiOuParams`
  - `HestonProcess`, `HestonParams` (mc)
  - `BatesProcess`, `BatesParams` (mc)
  - `CirProcess`, `CirParams`, `CirPlusPlusProcess` (mc)
  - `MertonJumpProcess`, `MertonJumpParams` (mc)
  - `HullWhite1FProcess`, `HullWhite1FParams` (mc)
  - `VasicekProcess` (mc)
  - `SchwartzSmithProcess`, `SchwartzSmithParams` (mc)
  - Helper fns: `apply_correlation`, `cholesky_decomposition`
- [ ] `bindings/monte_carlo/discretization.rs` → wrap schemes:
  - `ExactGbm`, `ExactMultiGbm`, `ExactMultiGbmCorrelated`
  - `EulerMaruyama`, `LogEuler`, `Milstein`, `LogMilstein` (mc)
  - `JumpEuler`, `QeHeston`, `QeCir` (mc)
  - `ExactHullWhite1F`, `ExactSchwartzSmith` (mc)
  - `CheyetteRoughEuler`, `RoughBergomiEuler`, `RoughHestonHybrid` (mc)
- [ ] `bindings/monte_carlo/payoff.rs` → wrap payoffs:
  - `EuropeanCall`, `EuropeanPut`, `Forward`, `Digital`
  - `AsianCall`, `AsianPut`, `AveragingMethod` (mc)
  - `BarrierOptionPayoff`, `BarrierType` (mc)
  - `BasketCall`, `BasketPut`, `BasketType`, `ExchangeOption` (mc)
  - `AmericanCall`, `AmericanPut` (mc)
- [ ] `bindings/monte_carlo/pricer.rs` → wrap pricers:
  - `EuropeanPricer`, `EuropeanPricerConfig`
  - `LsmcPricer`, `LsmcConfig` (mc)
  - `PathDependentPricer`, `PathDependentPricerConfig` (mc)
  - `AntitheticConfig`
  - `black_scholes_call`, `black_scholes_put`
- [ ] `bindings/monte_carlo/types.rs` → wrap data types:
  - `PathState`, `PathDataset`, `PathPoint`, `SimulatedPath`
  - `ProcessParams`, `CashflowType` (enum), `PathSamplingMethod` (enum)
  - `OnlineStats`, `OnlineCovariance`, `Estimate`
  - `StateKey` (enum), state_keys constants
  - `FractionalNoiseConfig`, `FbmGeneratorType` (mc)

**WASM work**: Mirror key types/functions (skip types that don't serialize well over wasm-bindgen).

**Artifacts**: `.pyi`, `__init__.py`, `exports/monte_carlo.js`, `index.d.ts`, tests.

---

## Phase 2: Infrastructure Domains

### 2A — Margin Full Coverage

**Gap**: Only VM + CSA + basic enums; entire IM stack, types, XVA missing.

**Python work** (split into subfiles under `bindings/margin/`):

- [ ] `bindings/margin/mod.rs` → orchestration
- [ ] `bindings/margin/types.rs` → wrap all type-level exports:
  - Enums: `ClearingStatus`, `CollateralAssetClass`, `ImMethodology`, `MarginCallType`, `MarginTenor`, `RepoMarginType`, `ScheduleAssetClass`, `SimmCreditSector`, `SimmRiskClass`, `CcpMethodology`, `SimmVersion`
  - Structs: `CsaSpec` (full field access), `NettingSetId`, `VmParameters`, `ImParameters`, `VmResult` (full), `ImResult` (full with breakdown)
  - Structs: `CollateralEligibility`, `ConcentrationBreach`, `EligibleCollateralSchedule`, `InstrumentMarginResult`, `MarginCall`, `MarginCallTiming`, `MaturityConstraints`, `OtcMarginSpec`, `RepoMarginSpec`, `SimmSensitivities`
- [ ] `bindings/margin/calculators.rs` → wrap calculator types:
  - `VmCalculator` (expand existing)
  - `SimmCalculator`, `ScheduleImCalculator`, `ClearingHouseImCalculator`, `HaircutImCalculator`, `InternalModelImCalculator`
  - Trait dispatch via spec/enum pattern for `ImCalculator`
- [ ] Consider: `xva` module bindings (if public API is stable)

**WASM work**: Mirror types + calculators.

**Artifacts**: `.pyi`, `__init__.py`, `exports/margin.js`, `index.d.ts`, tests.

### 2B — Statements Full Coverage

**Gap**: Only spec enums + JSON; need types, builder, evaluator, extensions.

**Python work** (split into subfiles under `bindings/statements/`):

- [ ] `bindings/statements/mod.rs` → orchestration
- [ ] `bindings/statements/types.rs` → wrap all crate-root type exports:
  - `AmountOrScalar` (enum), `CapitalStructureSpec`, `DebtInstrumentSpec` (enum), `FinancialModelSpec` (full field access beyond JSON)
  - `ForecastSpec`, `ForecastMethod` (expand existing), `NodeSpec`, `NodeId`, `NodeType`, `NodeValueType` (enum), `SeasonalMode` (enum), `NumericMode` (add Decimal variant)
- [ ] `bindings/statements/builder.rs` → wrap builder API:
  - `ModelBuilder`, `MixedNodeBuilder` (fluent builder pattern)
- [ ] `bindings/statements/evaluator.rs` → wrap evaluation:
  - `Evaluator`, `EvaluatorWithContext`, `PreparedEvaluation`, `StatementResult`
- [ ] `bindings/statements/extensions.rs` → wrap extension system:
  - `Extension` (trait), `ExtensionContext`, `ExtensionMetadata`, `ExtensionRegistry`, `ExtensionResult`, `ExtensionStatus`
  - `Registry`
- [ ] `bindings/statements/dates.rs` → expose prelude date helpers:
  - `build_periods`, `Period`, `PeriodId`, `PeriodKind`

**WASM work**: Focus on evaluation path (deserialize spec → evaluate → serialize result).

**Artifacts**: `.pyi`, `__init__.py`, `exports/statements.js`, `index.d.ts`, tests.

---

## Phase 3: Product Domains (depend on Phase 2)

### 3A — Valuations Full Coverage

**Gap**: JSON validation only; entire pricing stack missing.

**Python work** (split into subfiles under `bindings/valuations/`):

- [ ] `bindings/valuations/mod.rs` → orchestration
- [ ] `bindings/valuations/results.rs` → expand `ValuationResult` (full field access)
  - `ResultsMeta`
- [ ] `bindings/valuations/instruments.rs` → wrap all instrument types from prelude:
  - `Instrument`, `Attributes`, `InstrumentType`
  - Individual instruments: `Bond`, `BondConvention`, `InterestRateSwap`, `FixedLegSpec`, `FloatLegSpec`, `BasisSwap`, `CreditDefaultSwap`, `CDSIndex`, `CDSTranche`, `Deposit`, `EquityOption`, `FxForward`, `FxOption`, `FxSwap`, `InflationLinkedBond`, `Repo`, `RevolvingCredit`, `StructuredCredit`, `Swaption`, `TermLoan`, `VarianceSwap`, `AsianOption`, `BarrierOption`, `ConvertibleBond`
  - Enums: `ExerciseStyle`, `OptionType`, `PayReceive`, `SettlementType`
  - `PricingOptions`, `PricingOverrides`
- [ ] `bindings/valuations/pricer.rs` → wrap pricing:
  - `PricerRegistry`, `standard_registry()`
  - `ModelKey`
- [ ] `bindings/valuations/metrics.rs` → wrap metrics:
  - `MetricId`, `MetricRegistry`, `MetricContext`, `standard_registry` (metrics)
- [ ] `bindings/valuations/calibration.rs` → wrap calibration if stable public API
- [ ] `bindings/valuations/cashflow.rs` → re-export `finstack_cashflows` types
- [ ] `bindings/valuations/attribution.rs` → wrap attribution
- [ ] `bindings/valuations/covenants.rs` → wrap covenants
- [ ] Consider: `xva` module, `margin` submodule, `schema`

**WASM work**: Focus on pricing path (instrument JSON + market context → ValuationResult).

**Artifacts**: `.pyi`, `__init__.py`, `exports/valuations.js`, `index.d.ts`, tests.

### 3B — Statements Analytics Full Coverage

**Gap**: 4 JSON helpers only; missing orchestrator, DCF, credit, reports, MC, tornado, introspection.

**Python work** (split into subfiles under `bindings/statements_analytics/`):

- [ ] `bindings/statements_analytics/mod.rs` → orchestration
- [ ] `bindings/statements_analytics/analysis.rs` → wrap analysis functions/types:
  - `backtest_forecast` (expand existing), `ForecastMetrics`
  - `evaluate_dcf_with_market`, `CorporateValuationResult`, `DcfOptions`
  - `forecast_breaches`, `compute_credit_context`, `CreditContextMetrics`
  - `goal_seek`
  - `generate_tornado_entries`, `TornadoEntry`
  - `SensitivityAnalyzer`, `SensitivityConfig`, `SensitivityMode`, `SensitivityResult`, `ParameterSpec`
- [ ] `bindings/statements_analytics/orchestrator.rs`:
  - `CorporateAnalysis`, `CorporateAnalysisBuilder`, `CreditInstrumentAnalysis`
- [ ] `bindings/statements_analytics/variance.rs`:
  - `VarianceAnalyzer`, `VarianceConfig`, `VarianceReport`, `VarianceRow`
  - `BridgeChart`, `BridgeStep`
- [ ] `bindings/statements_analytics/scenarios.rs`:
  - `ScenarioDefinition`, `ScenarioDiff`, `ScenarioResults`, `ScenarioSet`
  - `MonteCarloConfig`, `MonteCarloResults`, `PercentileSeries`
- [ ] `bindings/statements_analytics/introspection.rs`:
  - `DependencyTracer`, `DependencyTree`, `FormulaExplainer`, `Explanation`, `ExplanationStep`
  - `render_tree_ascii`, `render_tree_detailed`
- [ ] `bindings/statements_analytics/reports.rs`:
  - `PLSummaryReport`, `CreditAssessmentReport`, `Report`, `TableBuilder`, `Alignment`
- [ ] `bindings/statements_analytics/extensions.rs`:
  - `CorkscrewExtension`, `CorkscrewConfig`, `CorkscrewAccount`, `AccountType`
  - `CreditScorecardExtension`, `ScorecardConfig`, `ScorecardMetric`
  - `RealEstateExtension`, `TemplatesExtension`, `VintageExtension`

**WASM work**: Mirror key functions and types.

**Artifacts**: `.pyi`, `__init__.py`, `exports/statements_analytics.js`, `index.d.ts`, tests.

### 3C — Portfolio Full Coverage

**Gap**: JSON helpers only; entire portfolio management stack missing.

**Python work** (split into subfiles under `bindings/portfolio/`):

- [ ] `bindings/portfolio/mod.rs` → orchestration
- [ ] `bindings/portfolio/types.rs` → wrap core types:
  - `Entity`, `EntityId`, `PositionId`, `DUMMY_ENTITY_ID`
  - `Position`, `PositionUnit` (enum)
  - `Portfolio`, `PortfolioSpec`
  - `Book`, `BookId`
  - `PortfolioResult`, `PositionValue`
- [ ] `bindings/portfolio/builder.rs`:
  - `PortfolioBuilder` (fluent builder)
- [ ] `bindings/portfolio/valuation.rs`:
  - `value_portfolio`, `revalue_affected`
  - `PortfolioValuation`, `PortfolioValuationOptions`
- [ ] `bindings/portfolio/metrics.rs`:
  - `aggregate_metrics` (expand existing), `AggregatedMetric`, `PortfolioMetrics`, `SkippedMetric`
- [ ] `bindings/portfolio/grouping.rs`:
  - `group_by_attribute`, `aggregate_by_attribute`, `aggregate_by_book`, `aggregate_by_multiple_attributes`
- [ ] `bindings/portfolio/attribution.rs`:
  - `attribute_portfolio_pnl`, `PortfolioAttribution`
- [ ] `bindings/portfolio/cashflows.rs`:
  - `aggregate_cashflows`, `cashflows_to_base_by_period`, `collapse_cashflows_to_base_by_date`
  - `PortfolioCashflows`, `PortfolioCashflowBuckets`, `PortfolioCashflowPositionSummary`
- [ ] `bindings/portfolio/margin.rs`:
  - `NettingSet`, `NettingSetManager`, `NettingSetMargin`, `PortfolioMarginAggregator`, `PortfolioMarginResult`
- [ ] `bindings/portfolio/factor_model.rs`:
  - `FactorModel` (trait), `FactorModelBuilder`, `RiskDecomposition`
- [ ] `bindings/portfolio/optimization.rs`:
  - `optimize_max_yield_with_ccc_limit`, `MaxYieldWithCccLimitResult`, `PortfolioOptimizationProblem`, `PortfolioOptimizationResult`
- [ ] `bindings/portfolio/dependencies.rs`:
  - `DependencyIndex`, `MarketFactorKey`
- [ ] `bindings/portfolio/scenarios.rs` (behind feature):
  - `apply_scenario`, `apply_and_revalue`

**WASM work**: Focus on valuation path + metrics + scenarios.

**Artifacts**: `.pyi`, `__init__.py`, `exports/portfolio.js`, `index.d.ts`, tests.

### 3D — Scenarios Full Coverage

**Gap**: Spec/template helpers only; execution engine not exposed.

**Python work** (split into subfiles under `bindings/scenarios/`):

- [ ] `bindings/scenarios/mod.rs` → orchestration
- [ ] `bindings/scenarios/spec.rs` → wrap spec types:
  - `ScenarioSpec` (full typed access, not just JSON)
  - `OperationSpec` (enum with all 25 variants)
  - `RateBindingSpec`
  - Enums: `Compounding`, `CurveKind`, `TenorMatchMode`, `TimeRollMode`, `VolSurfaceKind`, `HierarchyTarget`, `InstrumentType`
- [ ] `bindings/scenarios/engine.rs` → wrap execution:
  - `ScenarioEngine` (with `compose` + `apply` methods)
  - `ExecutionContext` (builder or direct construction)
- [ ] `bindings/scenarios/templates.rs` → expand template API:
  - `TemplateRegistry`, `RegisteredTemplate`, `TemplateMetadata`
  - `ScenarioSpecBuilder`
  - Enums: `AssetClass`, `Severity`
- [ ] `bindings/scenarios/error.rs`:
  - `Error` enum → `ScenarioError` Python exception

**WASM work**: Mirror engine.apply + typed spec + templates.

**Artifacts**: `.pyi`, `__init__.py`, `exports/scenarios.js`, `index.d.ts`, tests.

---

## Phase 4: Cross-Cutting

### 4A — Update Parity Contract

- [ ] Update `parity_contract.toml` with per-domain export counts
- [ ] Add audit script that reads contract + binding `__all__` and flags gaps

### 4B — Full Test Suite

- [ ] Expand Python tests per domain (target: parity test per crate-root export)
- [ ] WASM namespace tests (Node.js based)
- [ ] Performance benchmarks for new heavy paths

### 4C — Final Verification

- [ ] `cargo clippy -p finstack-py -- -D warnings` clean
- [ ] `cargo clippy -p finstack-wasm --target wasm32-unknown-unknown -- -D warnings` clean
- [ ] All Python tests pass
- [ ] All `.pyi` stubs pass `pyright` / `ty` check
- [ ] `wasm-pack build` succeeds

---

## Execution Order

```
Phase 1A (correlation) ──┐
Phase 1B (analytics)  ───┤── can run in parallel
Phase 1C (monte_carlo) ──┘
          │
Phase 2A (margin) ───────┐── can run in parallel
Phase 2B (statements) ───┘
          │
Phase 3A (valuations) ────── depends on 2B
Phase 3B (stmt_analytics) ── depends on 2B
Phase 3C (portfolio) ─────── depends on 3A
Phase 3D (scenarios) ─────── depends on 2B
          │
Phase 4 (cross-cutting) ──── depends on all above
```

## Estimated Scale

| Phase | New/Modified .rs Files | New Types | New Functions | Est. LOC |
|-------|----------------------|-----------|---------------|----------|
| 1A | ~4 | ~8 | ~4 | ~400 |
| 1B | ~8 (Py+WASM) | ~15 | ~80 | ~4,000 |
| 1C | ~12 (Py+WASM) | ~50 | ~20 | ~5,000 |
| 2A | ~6 (Py+WASM) | ~30 | ~10 | ~3,000 |
| 2B | ~10 (Py+WASM) | ~25 | ~15 | ~3,500 |
| 3A | ~14 (Py+WASM) | ~35 | ~10 | ~5,000 |
| 3B | ~14 (Py+WASM) | ~30 | ~15 | ~4,000 |
| 3C | ~14 (Py+WASM) | ~30 | ~15 | ~4,000 |
| 3D | ~8 (Py+WASM) | ~20 | ~10 | ~2,500 |
| 4 | tests + stubs | — | — | ~3,000 |
| **Total** | **~90** | **~243** | **~179** | **~34,400** |
