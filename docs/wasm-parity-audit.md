# Finstack WASM Bindings vs Rust Crate: Parity Audit

**Date:** 2026-02-28
**Scope:** Complete comparison of `finstack-wasm` against `finstack`, `finstack-core`, `finstack-valuations`, `finstack-statements`, `finstack-portfolio`, `finstack-scenarios`

## Executive Summary

The WASM bindings cover ~85-90% of the Rust crate surface area for core valuation/portfolio/scenario paths. The TypeScript interface is generally well-designed with camelCase naming, fluent builders, and static factories. However, there are meaningful parity gaps, documentation holes, and TypeScript type-safety issues that would frustrate a professional quant developer.

---

## 1. Parity Gaps

### 1.1 Entire Modules Missing

#### `finstack_core::analytics` — **0% coverage**

The `Performance` struct with 50+ metrics and all standalone functions are completely absent from WASM. This is the most critical gap — any JS quant doing portfolio analytics, performance attribution, or risk reporting has zero access.

**Missing APIs:**
- `Performance` struct (Sharpe, Sortino, Calmar, VaR, ES, drawdowns, rolling stats, capture ratios, beta, greeks, 50+ methods)
- Standalone: `simple_returns`, `excess_returns`, `cagr`, `volatility`, `mean_return`, `sharpe`, `sortino`, `calmar`, `downside_deviation`, `value_at_risk`, `expected_shortfall`, `skewness`, `kurtosis`, `geometric_mean`, `omega_ratio`, `treynor`, `gain_to_pain`, `martin_ratio`, `parametric_var`, `cornish_fisher_var`, `recovery_factor`, `sterling_ratio`, `burke_ratio`, `pain_index`, `pain_ratio`, `m_squared`, `modified_sharpe`
- Rolling: `rolling_sharpe`, `rolling_volatility`, `rolling_sortino`, `rolling_greeks`
- Drawdown: `DrawdownEpisode`, `to_drawdown_series`, `drawdown_details`, `avg_drawdown`, `max_drawdown_duration`, `cdar`
- Benchmark: `tracking_error`, `information_ratio`, `r_squared`, `calc_beta`, `greeks`, `up_capture`, `down_capture`, `capture_ratio`, `batting_average`, `multi_factor_greeks`
- Returns: `clean_returns`, `simple_returns`, `excess_returns`, `convert_to_prices`, `rebase`, `comp_sum`, `comp_total`
- Aggregation: `PeriodStats`, `group_by_period`, `period_stats`
- Lookback: `LookbackReturns`

#### `finstack_core::dates::rate_conversions` — **0% coverage**

- `simple_to_periodic`, `periodic_to_simple`, `periodic_to_continuous`, `continuous_to_periodic`, `simple_to_continuous`, `continuous_to_simple`

#### `finstack_valuations::xva` — **0% coverage**

- `XvaConfig`, `XvaResult`, `ExposureProfile`, `NettingSet` (XVA-specific), `CsaTerms`, `compute_cva`, `compute_exposure_profile`

---

### 1.2 Missing Instruments (3)

| Instrument | Rust Module |
|------------|-------------|
| `FxDigitalOption` | `finstack_valuations::instruments::fx_digital_option` |
| `FxTouchOption` | `finstack_valuations::instruments::fx_touch_option` |
| `CommodityAsianOption` | `finstack_valuations::instruments::commodity_asian_option` |

---

### 1.3 Missing Metric IDs (~110 of 150+)

Only ~40 metric IDs exposed via `JsMetricId`. Notable gaps by asset class:

| Category | Missing Metrics |
|----------|----------------|
| **FX** | SpotRate, BaseAmount, QuoteAmount, InverseRate |
| **Equity** | EquityPricePerShare, EquityShares, EquityDividendYield, EquityForwardPrice |
| **CDS** | RiskyPv01, RiskyAnnuity, ProtectionLegPv, PremiumLegPv, JumpToDefault, ExpectedLoss |
| **Deposit** | Yf, DfStart, DfEnd, DepositParRate, QuoteRate |
| **Options** | BucketedVega, ForeignRho, CsGamma, InflationConvexity |
| **Risk sensitivities** | Dividend01, Inflation01, Prepayment01, Default01, Fx01, SpreadDv01, Correlation01, FxVega, ConvexityAdjustmentRisk |
| **Basis swap** | PvPrimary, PvReference, AnnuityPrimary, AnnuityReference, Dv01Primary, Dv01Reference, BasisParSpread, IncrementalParSpread |
| **Repo** | CollateralValue, RequiredCollateral, CollateralCoverage, RepoInterest, EffectiveRate, ImpliedCollateralReturn |
| **Structured credit** | WAL, WAM, ExpectedMaturity, PoolFactor, CPR, SMM, CDR, LossSeverity, SpreadDuration, Dm01 |
| **ABS/CLO/CMBS/RMBS** | CloWarf, CloWas, CloWac, CloDiversity, CloOcRatio, CloIcRatio, CmbsDscr, CmbsWaltv, RmbsPsaSpeed, RmbsSdaSpeed (20+) |
| **Inflation-linked** | RealYield, IndexRatio, RealDuration, BreakevenInflation |
| **Private markets** | LpIrr, GpIrr, MoicLp, DpiLp, TvpiLp, CarryAccrued |
| **DCF** | EnterpriseValue, EquityValue, TerminalValuePV |
| **Core** | Pv01, BucketedCs01, ThetaGamma, HVAR, EXPECTED_SHORTFALL |

---

### 1.4 Missing Statements APIs

| API | Description |
|-----|-------------|
| `MonteCarloConfig`, `MonteCarloResults` | Monte Carlo simulation for financial models |
| `goal_seek` | Goal-seeking / solver for model parameters |
| `SensitivityAnalyzer`, `SensitivityConfig`, `SensitivityResult` | Parameter sensitivity analysis |
| `VarianceAnalyzer`, `VarianceReport`, `BridgeChart` | Variance and bridge analysis |
| `CorporateAnalysis`, `CorporateAnalysisBuilder` | Orchestrated corporate analysis |
| `FormulaExplainer` | Formula explanation / audit trail |
| `CreditContextMetrics`, `compute_credit_context` | Credit context computation |
| `forecast_breaches` | Covenant breach forecasting |
| `Report`, `TableBuilder`, `PLSummaryReport`, `CreditAssessmentReport` | Reporting / table generation |
| `AliasRegistry` | Metric aliasing |
| Template APIs | `add_roll_forward`, `add_vintage_buildup`, `add_noi_buildup`, `add_ncf_buildup`, `add_rent_roll_rental_revenue`, `add_property_operating_statement` |

---

### 1.5 Missing Portfolio APIs

| API | Description |
|-----|-------------|
| `PortfolioOptimizationProblem` | General optimization problem definition |
| `DefaultLpOptimizer` | LP optimizer |
| `CandidatePosition`, `TradeUniverse`, `TradeSpec` | Trade/position candidates |
| `Constraint`, `Objective`, `MetricExpr`, `PerPositionMetric` | Optimization constraints & objectives |
| `PositionFilter`, `WeightingScheme` | Filtering and weighting |

Only `optimizeMaxYieldWithCccLimit` is exposed as a convenience function.

---

### 1.6 Missing Margin/IM APIs

| API | Description |
|-----|-------------|
| `ImCalculator` trait, `ImResult` | Initial margin calculation interface |
| `ScheduleImCalculator`, `MarginRegistry` | Schedule-based IM, registry |
| SIMM sensitivities | ISDA SIMM framework |

Only VM (variation margin) and CSA-related types are exposed.

---

### 1.7 Missing Top-Level Exports

- `isValidBusinessDayConvention` — exists in `calendar.rs` but not re-exported from `lib.rs`
- `allBusinessDayConventions` — exists in `calendar.rs` but not re-exported from `lib.rs`
- `InterpStyle` — defined in `interp.rs` but not exported (only used internally)
- `ExtrapolationPolicy` — defined in `interp.rs` but not exported

---

## 2. TypeScript Interface Issues

### 2.1 Type Safety: `any` Usage (~103 instances)

| Category | Count | Examples | Recommended Fix |
|----------|-------|----------|-----------------|
| JSON serialization | ~80 | `toJson(): any`, `fromJson(value: any)` | Use `unknown` or typed interfaces |
| Callback functions | ~5 | `BrentSolver.solve(func: any, ...)` | `(x: number) => number` |
| Arrays | ~15+ | `getCashflows(): Array<any>`, `Currency.all(): Array<any>` | `CashFlow[]`, `Currency[]` |
| Untyped params | ~10 | `DiscountCurve(day_count: any, interp: any)` | Use `DayCount`, `InterpStyle` |

### 2.2 Naming Inconsistency: `fromJson` vs `fromJSON`

- **63 uses** of `fromJSON` (JavaScript convention)
- **100 uses** of `fromJson` (camelCase convention)

Mixed across the API. Should standardize on one convention.

### 2.3 Constructor Naming Inconsistencies

| Pattern | Examples | Issue |
|---------|----------|-------|
| `new_from_name` | `DayCount`, `StubKind` | Rust-ism leaking through; should be `fromName()` |
| `new_config` | `FiscalConfig` | Inconsistent with `new()` elsewhere |
| Long positional args | `ScheduleSpec` (10 args) | Should use options object |

---

## 3. Documentation Gaps

### 3.1 Well-Documented (good JSDoc with `@param`, `@returns`, `@example`)

- `Currency`, `Money`, `Rate`, `Bps`, `Percentage`
- `Bond`, `Deposit`, `InterestRateSwap`, `CreditDefaultSwap`
- `DayCount.yearFraction`, `MetricId`, `PricerRegistry`
- `pricer.rs`, `performance.rs`, `dataframe.rs`
- Math: solvers, probability, special functions

### 3.2 Missing JSDoc

| Module | Items Needing Docs |
|--------|--------------------|
| **Calendar** | `Calendar`, `adjust`, `availableCalendars`, `getCalendar`, `BusinessDayConvention` |
| **Schedule** | `ScheduleBuilder`, `Schedule`, `ScheduleSpec`, `StubKind` |
| **Periods** | `PeriodId`, `Period`, `PeriodPlan`, `FiscalConfig`, `buildPeriods`, `buildFiscalPeriods` |
| **IMM dates** | `nextImm`, `nextCdsDate`, `nextImmOptionExpiry`, `thirdFriday`, `thirdWednesday` |
| **Date utils** | `addMonths`, `lastDayOfMonth`, `daysInMonth`, `isLeapYear` |
| **FX** | `FxMatrix`, `FxConversionPolicy`, `FxConfig`, `FxRateResult` |
| **Dividends** | `DividendEvent`, `DividendSchedule`, `DividendScheduleBuilder` |
| **Scalars** | `MarketScalar`, `ScalarTimeSeries`, `SeriesInterpolation` |
| **Config** | `RoundingMode` |
| **Risk** | `VarConfig`, `VarResult`, `MarketHistory`, `MarketScenario`, `RiskFactorShift` |
| **Conventions** | `CdsConventions`, `SwaptionConventions`, `InflationSwapConventions`, `OptionConventions` |
| **Most builders** | `BondBuilder`, `CashflowBuilder`, and other `XxxBuilder` classes |

---

## 4. Prioritized Work Plan

### P0 — Ship-blocking for quant developers

- [ ] **P0-1:** Expose `finstack_core::analytics` module (Performance struct + standalone functions)
- [ ] **P0-2:** Expose remaining 110+ MetricIds
- [ ] **P0-3:** Add 3 missing instruments (FxDigitalOption, FxTouchOption, CommodityAsianOption)

### P1 — Developer experience / type safety

- [ ] **P1-1:** Replace `any` with proper types in `.d.ts` generation
  - [ ] `getCashflows()` → `CashFlow[]`
  - [ ] Solver/integration callbacks → `(x: number) => number`
  - [ ] Curve constructor params → `DayCount`, `InterpStyle`, `ExtrapolationPolicy`
  - [ ] JSON serialization → `unknown` at minimum
- [ ] **P1-2:** Standardize `fromJson`/`fromJSON` naming (pick one, fix all 163 occurrences)
- [ ] **P1-3:** Add JSDoc to all undocumented modules (Calendar, Schedule, Periods, IMM, FX, Dividends, Scalars, Risk, Conventions, Builders)
- [ ] **P1-4:** Expose `rate_conversions` (6 functions)
- [ ] **P1-5:** Export `isValidBusinessDayConvention`, `allBusinessDayConventions` from top level
- [ ] **P1-6:** Export `InterpStyle` and `ExtrapolationPolicy` as first-class types

### P2 — Feature completeness

- [ ] **P2-1:** Expose XVA module (CVA, exposure profiles)
- [ ] **P2-2:** Expose Statements analysis APIs (Monte Carlo, sensitivity, variance, goal seek, corporate analysis)
- [ ] **P2-3:** Expose Portfolio optimization APIs (general problem definition, constraints, objectives)
- [ ] **P2-4:** Expose IM/SIMM margin APIs
- [ ] **P2-5:** Expose template APIs for financial model construction

### P3 — Polish

- [ ] **P3-1:** Fix constructor naming inconsistencies (`new_from_name` → `fromName`)
- [ ] **P3-2:** Convert long positional constructors to options objects (e.g. `ScheduleSpec`)
- [ ] **P3-3:** Add `@example` blocks to the 20 most-used APIs
- [ ] **P3-4:** Audit attribution types (some are `#[allow(dead_code)]` and not exported)

---

## 5. Coverage Summary

| Crate / Module | Coverage | Notes |
|----------------|----------|-------|
| finstack-core (dates, cashflow, config, currency, money, types, math, market_data, expr) | ~85% | Missing: rate_conversions, some utility exports |
| **finstack-core (analytics)** | **0%** | Entire Performance struct + 50 standalone functions |
| finstack-valuations (instruments) | ~95% | 3 instruments missing |
| **finstack-valuations (metrics)** | **~25%** | Only ~40 of 150+ metric IDs exposed |
| finstack-valuations (calibration, pricer, cashflow) | ~90% | Good coverage |
| **finstack-valuations (xva)** | **0%** | Entire module missing |
| finstack-valuations (margin) | ~50% | VM exposed; IM/SIMM missing |
| finstack-statements (core) | ~90% | Builder, evaluator, extensions well covered |
| finstack-statements (analysis) | ~30% | Monte Carlo, sensitivity, variance, reports missing |
| finstack-portfolio (core) | ~90% | Positions, valuation, cashflows, margin well covered |
| finstack-portfolio (optimization) | ~15% | Only one convenience function |
| finstack-scenarios | ~95% | Near-complete |

---

## 6. What Works Well

1. **Naming:** Consistent camelCase functions, PascalCase types
2. **Builder patterns:** Fluent `.method().method().build()` throughout
3. **Static factories:** `Rate.fromDecimal()`, `Rate.fromPercent()`, `Rate.fromBps()`
4. **Enum patterns:** Static factory methods (`CFKind.Fixed()`, `OptionType.call()`)
5. **Serialization:** `toJson()`/`fromJson()` round-trip on most types
6. **Flat exports:** Simple imports without deep module paths
7. **Examples:** 114+ TypeScript example files in `examples/`
8. **Docs style guide:** `DOCS_STYLE.md` establishes `@param`, `@returns`, `@example` standard
