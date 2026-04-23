# Rust Crate Documentation Review

**Date:** 2026-04-22  
**Scope:** All Rust crates in `finstack/` workspace  
**Method:** Automated scan of `pub fn/struct/enum/trait/type` items against project documentation standards.

---

## Summary

| Metric | Count |
|--------|-------|
| Total `pub` items scanned | ~7,400 |
| **Blocker:** Items with **zero** documentation | **466** across 193 files |
| **Major:** Documented items missing required sections | **4,785** across 869 files |
| `no # Arguments` | 2,890 |
| `no # Returns` | 2,981 |
| `no # Examples` (no code block) | 4,318 |
| `no # References` (on financial/math code) | 983 |

### Coverage by Crate

| Crate | Files with Issues | Items with Incomplete Docs | Items with No Docs |
|-------|-------------------|---------------------------|-------------------|
| `valuations` | 422 | 2,163 | 132 |
| `core` | 147 | 1,001 | 32 |
| `monte_carlo` | 57 | 353 | 3 |
| `portfolio` | 46 | 288 | 2 |
| `margin` | 45 | 243 | 8 |
| `statements` | 40 | 206 | 7 |
| `statements-analytics` | 41 | 191 | 1 |
| `analytics` | 26 | 148 | 2 |
| `cashflows` | 28 | 115 | 6 |
| `scenarios` | 17 | 77 | 0 |

**No `io` crate was found in the workspace** (referenced in `project-description.md` but not present as a standalone crate).

---

## Missing Documentation

### Blockers — Public API with no documentation at all

These are `pub fn/struct/enum/trait/type` declarations with **no** `///` or `//!` comment anywhere in scope.

#### `valuations` (132 items)
- `valuations:results/valuation_result.rs` — `stamped`, `stamped_with_config`, `stamped_with_meta`, `with_explanation`, `with_measures`, `metric`, `metric_str`, `get_measure`, `with_covenants`, `with_covenant`
- `valuations:pricer/registry.rs` — `expect_inst`, `Pricer`, `PricerRegistry`, `new`, `register_pricer`, `register`, `get_pricer`, `get`
- `valuations:pricer/mod.rs` — `register_rates_pricers`, `register_credit_pricers`, `register_equity_pricers`, `register_fx_pricers`, `register_fixed_income_pricers`, `register_inflation_pricers`, `register_exotic_pricers`, `register_commodity_pricers`, `standard_registry`, `shared_standard_registry`
- `valuations:pricer/json.rs` — `parse_instrument_json`, `validate_instrument_json`, `parse_boxed_instrument_json`, `parse_as_of_date`, `parse_model_key`, `price_instrument_json`, `price_instrument_json_with_metrics`
- `valuations:pricer/keys.rs` — `InstrumentType`, `PricerKey`
- `valuations:restructuring/types.rs` — `ClaimSeniority`, `Claim`, `total_claim`, `CollateralAllocation`, `net_value`, `AllocationMode`
- `valuations:restructuring/recovery_waterfall.rs` — `RecoveryWaterfall`, `PlanDeviation`, `RecoveryResult`, `ClaimRecovery`, `execute_recovery_waterfall`
- `valuations:restructuring/lme.rs` — `LmeType`, `LmeSpec`, `LmeAnalysis`, `LeverageImpact`, `RemainingHolderImpact`, `analyze_lme`
- `valuations:restructuring/exchange_offer.rs` — `ExchangeType`, `ExchangeOffer`, `ExchangeInstrument`, `CouponPaymentType`, `EquitySweetener`, `EquityComponentType`, `HoldVsTenderAnalysis`, `ScenarioEconomics`, `TenderRecommendation`, `ConsentTracker`
- `valuations:reporting/cashflow_ladder.rs` — `BucketFrequency`, `CashflowBucket`, `from_cashflows`
- `valuations:reporting/format.rs` — `format_bps`, `format_pct`, `format_currency`, `format_scientific`, `format_ratio`, `SparklineData`, `sparkline_buckets`, `PercentileBadge`, `percentile_badge`
- `valuations:reporting/metrics_table.rs` — `Direction`, `MetricUnit`, `MetricRow`, `from_valuation_result`
- `valuations:reporting/mod.rs` — `ReportComponent`
- `valuations:reporting/scenario_matrix.rs` — `from_scenario_results`
- `valuations:reporting/sensitivity_grid.rs` — `from_sensitivity_matrix`
- `valuations:reporting/waterfall.rs` — `WaterfallStep`, `from_attribution`
- `valuations:xva/mod.rs` — `compute_exposure_profile`
- `valuations:utils/decimal.rs` — `f64_to_decimal`
- `valuations:schema.rs` — `bond_schema`, `instrument_envelope_schema`, `instrument_types`, `instrument_schema`, `valuation_result_schema`, `validate_instrument_envelope_json`, `validate_instrument_json`, `validate_instrument_type_json`
- Plus many more in `metrics/`, `pricer/`, `instruments/`

#### `core` (32 items)
- `core:cashflow/primitives.rs` — `CFKind`, `CashFlow`
- `core:config.rs` — `RoundingMode`
- `core:credit/lgd/seniority.rs` — `SeniorityClass`
- `core:credit/lgd/workout.rs` — `CollateralType`
- `core:dates/calendar/business_days.rs` — `CalendarMetadata`, `BusinessDayConvention`
- `core:dates/calendar/types.rs` — `WeekendRule`
- `core:dates/daycount.rs` — `DayCount`
- `core:dates/imm.rs` — `SifmaSettlementClass`
- `core:dates/periods.rs` — `PeriodKind`
- `core:dates/tenor.rs` — `TenorUnit`, `Tenor`
- `core:market_data/arbitrage/types.rs` — `ArbitrageSeverity`
- `core:market_data/term_structures/hazard_curve.rs` — `Seniority`, `ParInterp`
- `core:math/interp/types.rs` — `ExtrapolationPolicy`
- `core:market_data/surfaces/mod.rs` — `recover_fx_wing_vols`, `fx_forward`, `fx_atm_dns_strike`
- `core:market_data/bumps.rs` — `id_bump_bp`, `id_spread_bp`, `id_bump_pct`

#### `margin` (8 items)
- `margin:types/call.rs` — `MarginCallType`
- `margin:types/enums.rs` — `MarginTenor`, `ImMethodology`, `ClearingStatus`
- `margin:types/netting.rs` — `NettingSetId`
- `margin:types/repo_margin.rs` — `RepoMarginType`
- `margin:types/simm_types.rs` — `SimmRiskClass`, `SimmCreditSector`

#### `statements` (7 items)
- `statements:models/corkscrew.rs` — `CorkscrewSchedule`
- `statements:models/drivers.rs` — `DriverValue`, `DriverFormula`
- `statements:models/evaluation.rs` — `EvaluationError`, `Evaluate`

#### `cashflows` (6 items)
- `cashflows:builder/specs/amortization.rs` — `AmortizationSpec`
- `cashflows:builder/specs/coupon.rs` — `CouponType`
- `cashflows:builder/specs/fees.rs` — `FeeAccrualBasis`
- `cashflows:accrual.rs` — `AccrualMethod`

#### `analytics` (2 items)
- `analytics:backtesting/types.rs` — `new`, `with_confidence`, `with_window_size`
- `analytics:comps/types.rs` — `new`, `as_str`

#### `portfolio` (2 items)
- `portfolio:valuation/types.rs` — `ValuationSummary`

#### `monte_carlo` (3 items)
- `monte_carlo:estimate.rs` — `Estimate`

#### `statements-analytics` (1 item)
- `statements-analytics:types.rs` — `CovenantStatus`

---

### Majors — Missing Arguments, Returns, or Examples

These items have a doc comment but lack required sections.

#### Top patterns

| Pattern | Approximate Count | Most Affected Crates |
|---------|--------------------|----------------------|
| No `# Examples` | 4,318 | valuations, core, monte_carlo |
| No `# Returns` | 2,981 | valuations, core, portfolio |
| No `# Arguments` | 2,890 | valuations, core, monte_carlo |
| No `# References` | 983 | valuations, core, monte_carlo |

#### Representative gaps by crate

**`valuations` (2,163 items)**
- `valuations:pricer/fourier/cos.rs` — `CosConfig`, `CosPricer`, `price_call`, `price_put`, `price_calls`, `price_puts` all missing Examples and References. Fourier pricing should cite Heston (1993) or Fang & Oosterlee.
- `valuations:metrics/sensitivities/fd_greeks.rs` — `HasPricingOverrides`, `GenericFdDelta`, `GenericFdGamma`, `GenericFdVega`, `GenericFdVolga`, `GenericFdVanna` missing References. Finite-difference Greeks should cite Hull.
- `valuations:metrics/sensitivities/breakeven.rs` — `sensitivity_metric` missing Arguments, Returns, Examples.
- `valuations:instruments/mod.rs` — Most instrument types missing Examples.
- `valuations:results/dataframe.rs` — `ValuationRow`, `to_row`, `to_rows`, `results_to_rows` missing Examples.

**`core` (1,001 items)**
- `core:cashflow/xirr.rs` — `irr`, `xirr`, `xirr_with_daycount`, `xirr_with_daycount_ctx`, `IrrResult`, `count_sign_changes`, `irr_detailed`, `xirr_detailed` all missing Arguments, Returns, Examples. XIRR is financial math and should reference Newton-Raphson or Brent methods.
- `core:credit/pd/term_structure.rs` — `PdTermStructure`, `cumulative_pd`, `marginal_pd`, `hazard_rate`, `tenors`, `cumulative_pds` missing Examples.
- `core:credit/scoring/altman.rs` — `AltmanZScoreInput`, `AltmanZPrimeInput`, `AltmanZDoublePrimeInput`, `altman_z_score`, `altman_z_prime`, `altman_z_double_prime` missing Examples. Should reference Altman (1968).
- `core:credit/scoring/ohlson.rs` — `OhlsonOScoreInput`, `ohlson_o_score` missing Examples. Should reference Ohlson (1980).
- `core:dates/calendar/business_days.rs` — `adjust` missing Arguments, Returns.
- `core:config.rs` — `FinstackConfig`, `ConfigExtensions`, `RoundingPolicy`, `RoundingContext`, `ZeroKind` and accessors missing Examples.

**`monte_carlo` (353 items)**
- `monte_carlo:payoff/mod.rs` — Re-exports (`AsianCall`, `BarrierOptionPayoff`, `BasketCall`, `EuropeanCall`, etc.) missing module-level or item-level Examples.
- `monte_carlo:pricer/mod.rs` — `european::*`, `lsmc::*`, `path_dependent::*` re-exports missing Examples.
- `monte_carlo:discretization/mod.rs` — `ExactGbmWithDividends`, `ExactHullWhite1F`, `EulerMaruyama`, `Milstein`, `QeCir`, `QeHeston`, etc. missing Examples.
- `monte_carlo:process/mod.rs` — `BatesProcess`, `CirProcess`, `HestonProcess`, `MertonJumpProcess` etc. missing Examples.

**`portfolio` (288 items)**
- `portfolio:valuation/types.rs` — `ValuationSummary`, `valuation_summary` missing Examples.
- `portfolio:aggregation/mod.rs` — Aggregation functions missing Arguments, Returns, Examples.

**`statements` (206 items)**
- `statements:models/evaluation.rs` — `EvaluationError`, `Evaluate` trait missing Examples.
- `statements:models/corkscrew.rs` — `CorkscrewSchedule` missing Examples.

**`statements-analytics` (191 items)**
- `statements-analytics:covenant.rs` — Covenant-related types and functions missing Examples.
- `statements-analytics:alignment.rs` — Alignment functions missing Examples.

**`margin` (243 items)**
- `margin:types/collateral.rs` — `MaturityConstraints` missing Examples.
- `margin:types/simm_types.rs` — `SimmRiskClass`, `SimmCreditSector` missing Examples.

**`cashflows` (115 items)**
- `cashflows:builder/specs/schedule.rs` — `ScheduleParams`, `quarterly_act360`, `semiannual_30360`, `annual_actact`, `usd_sofr_swap`, `usd_corporate_bond`, `usd_treasury`, `eur_estr_swap`, `eur_gov_bond`, `gbp_sonia_swap` missing Arguments, Returns, Examples. These are financial conventions and should reference ISDA or market standards.
- `cashflows:builder/specs/prepayment.rs` — `PrepaymentCurve`, `PrepaymentModelSpec`, `smm`, `constant_cpr`, `constant_cpr_pct`, `psa`, `psa_100`, `cmbs_with_lockout` missing Examples.
- `cashflows:builder/specs/default.rs` — `DefaultCurve`, `DefaultModelSpec`, `mdr`, `constant_cdr`, `sda`, `cdr_2pct` missing Examples.

**`analytics` (148 items)**
- `analytics:risk_metrics/mod.rs` — `value_at_risk`, `parametric_var`, `cornish_fisher_var` missing Examples and References. VaR methods should cite Jorion or Hull.
- `analytics:drawdown/mod.rs` — Drawdown functions missing Examples.
- `analytics:benchmark/mod.rs` — Regression and attribution functions missing Examples.

**`scenarios` (77 items)**
- `scenarios:types.rs` — Scenario types missing Examples.
- `scenarios:engine.rs` — Engine functions missing Examples.

---

### Minors — Missing references on financial code

- `valuations:pricer/fourier/cos.rs` — Fourier COS method should reference Fang & Oosterlee (2008).
- `valuations:metrics/sensitivities/fd_greeks.rs` — Finite-difference Greeks should reference Hull, Options, Futures, and Other Derivatives.
- `core:cashflow/xirr.rs` — XIRR root-finding should reference Brent (1973) or Newton-Raphson.
- `core:credit/scoring/altman.rs` — Should reference Altman, E. I. (1968). "Financial Ratios, Discriminant Analysis and the Prediction of Corporate Bankruptcy."
- `analytics:risk_metrics/mod.rs` — VaR/ES should reference Jorion, P. (2006). *Value at Risk*.
- `cashflows:builder/specs/schedule.rs` — Day-count and convention helpers should reference ISDA definitions.
- `monte_carlo:engine/mod.rs` — Monte Carlo engine should reference Glasserman (2003) or Boyle et al.

---

## Structural / Process Findings

1. **`missing_docs` lint not enabled in all crates**  
   `finstack_analytics` and `finstack_monte_carlo` do **not** have `#![warn(missing_docs)]` in their `lib.rs`. Every other crate does. Add it to both.

2. **`pub(crate)` and `pub(super)` items are undocumented**  
   These are not public API per Rust's visibility rules, but many lack even minimal comments. Internal maintainability suffers.

3. **Builder pattern types are under-documented**  
   Across crates, `new()`, builder setters (`with_*`), and `build()` methods often have no docs. These are primary user-facing entry points.

4. **Re-export modules (`mod.rs`) lack item-level docs**  
   Many `pub use` re-exports in module files have no individual `///` lines, relying only on module-level `//!` docs.

5. **Error types lack Examples**  
   Error enums and result types across all crates typically have descriptions but no `/// # Examples` showing how they are constructed or handled.

6. **Test-only modules are not gated from doc checks**  
   `#[cfg(test)]` modules with `pub` items (e.g., `tests.rs` files) show up in scans but are not user-facing. They should either be fully private or documented minimally.

---

## Recommendations

### Immediate (High Priority)

1. **Add `#![warn(missing_docs)]` to `analytics/src/lib.rs` and `monte_carlo/src/lib.rs`**  
   This brings them in line with every other crate and surfaces new gaps in CI.

2. **Fix all 466 Blocker items (zero docs)**  
   Focus on the highest-visibility public API first:
   - `valuations:pricer/*` — registry, keys, JSON parsing, instrument type enum
   - `valuations:results/*` — valuation result accessors
   - `valuations:restructuring/*` — exchange offer, LME, recovery waterfall types
   - `valuations:reporting/*` — formatting, tables, sparklines
   - `core:cashflow/*` — `CFKind`, `CashFlow`
   - `core:dates/*` — `DayCount`, `BusinessDayConvention`, `Tenor`
   - `core:config.rs` — `RoundingMode`
   - `margin:types/*` — all enums (`MarginTenor`, `ImMethodology`, `ClearingStatus`, etc.)

3. **Document builder entry points**  
   Every `new()`, `builder()`, and `build()` method should have at minimum a one-liner `///` doc. These are the first things users call.

### Short-Term (Next Sprint)

4. **Add `# Examples` to high-traffic modules**  
   Prioritize by user-facing surface area:
   - `valuations:pricer/*` — 2,163 items; most user-facing
   - `core:cashflow/*` — discounting, XIRR, cashflow primitives
   - `analytics:risk_metrics/*` — VaR, drawdown, Sharpe
   - `monte_carlo:engine`, `process`, `payoff` — entry points
   - `statements:models/*` — evaluation, corkscrew, drivers

5. **Add `# References` to financial/math code**  
   Any module implementing pricing models, Greeks, day-count conventions, VaR, Monte Carlo, or credit scoring should cite canonical sources per `docs/REFERENCES.md`.

6. **Add `# Arguments` and `# Returns` to functions**  
   Any `pub fn` should document its inputs and output. This is the second-most common gap after Examples.

### Medium-Term

7. **Establish a documentation coverage gate**  
   Consider upgrading workspace `missing_docs` from `warn` to `deny` once blockers are cleared. This prevents regressions.

8. **Add module-level `//!` docs where missing**  
   Some `mod.rs` files have only re-exports with no module-level explanation. Add a brief `//!` describing the module's purpose and key types.

9. **Document `pub(crate)` / `pub(super)` helpers**  
   Internal types need at least a one-liner so future maintainers understand intent without reading implementation.

10. **Create a documentation sweep script**  
    The audit script used for this review (`/tmp/doc_scan.py`) can be adapted into a CI check or pre-commit hook to track documentation coverage over time.

---

## Action Items Checklist

- [ ] Add `#![warn(missing_docs)]` to `finstack_analytics/src/lib.rs`
- [ ] Add `#![warn(missing_docs)]` to `finstack_monte_carlo/src/lib.rs`
- [ ] Document all 132 zero-doc items in `valuations`
- [ ] Document all 32 zero-doc items in `core`
- [ ] Document all 8 zero-doc items in `margin`
- [ ] Document all 7 zero-doc items in `statements`
- [ ] Document all 6 zero-doc items in `cashflows`
- [ ] Add `# Examples` to `valuations:pricer/*`, `valuations:results/*`, `valuations:reporting/*`
- [ ] Add `# Examples` to `core:cashflow/xirr.rs`, `core:cashflow/primitives.rs`
- [ ] Add `# References` to `valuations:pricer/fourier/cos.rs`
- [ ] Add `# References` to `valuations:metrics/sensitivities/fd_greeks.rs`
- [ ] Add `# References` to `core:credit/scoring/altman.rs`
- [ ] Add `# References` to `analytics:risk_metrics/mod.rs`
- [ ] Add `# Arguments` / `# Returns` to builder methods (`new`, `with_*`, `build`) across all crates
- [ ] Add `# Arguments` / `# Returns` to `core:dates/calendar/business_days.rs` `adjust`
- [ ] Consider `missing_docs` → `deny` after blocker clearance
