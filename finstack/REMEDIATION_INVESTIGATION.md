# Finstack Unfinished Work: Investigation & Remediation Guide

For each item from the inventory, this document provides:
1. **Current limitation** – what is missing or broken
2. **Remediation** – short description of fix/refactor to finish or improve

---

## 1. TODOs (Actionable)

### 1.1 Inflation-linked coupon emission

**Location:** `valuations/src/cashflow/builder/emission/coupons.rs:5`

**Current limitation:** Only fixed and floating coupons are emitted. CPI-linked coupons, index ratios, real/nominal decomposition, and multi-index support (CPI-U, HICP, RPI) are not implemented.

**Remediation:** Add `emit_inflation_coupons()` alongside `emit_fixed_coupons()` and `emit_float_coupons()`. Introduce `InflationSchedule` (analogous to `FixedSchedule`/`FloatSchedule`) with index_id, lag convention (2M/3M), and interpolation. Compute index ratios via `InflationIndex::ratio(as_of, fixing_date)` and emit `CFKind::InflationCoupon`. Wire into `InflationLinkedBond` and `YoYInflationSwap` cashflow builders.

---

### 1.2 WAL flow-type metadata

**Location:** `valuations/src/instruments/fixed_income/structured_credit/metrics/pricing/wal.rs:83`

**Current limitation:** When detailed tranche cashflows are unavailable, WAL falls back to aggregated flows. Interest and principal are combined, so using absolute value overstates WAL (interest is incorrectly treated as principal).

**Remediation:** Extend `context.cashflows` (or introduce `context.cashflows_tagged`) to carry `CFKind` or a `flow_type` enum per flow. In the fallback path, filter to principal-only flows before computing WAL. Alternatively, ensure `detailed_tranche_cashflows` is populated for all structured credit metrics so the fallback is rarely used.

---

### 1.3 Make-whole call in bond tree pricer

**Location:** `valuations/src/instruments/fixed_income/bond/pricing/engine/tree.rs:850`

**Current limitation:** When `call.make_whole` is `Some(spec)`, the effective call price should be `max(price_pct_of_par, PV(remaining cashflows at reference_rate + spread))`. Currently only `price_pct_of_par` is used.

**Remediation:** Inside the tree backward induction, at each call step: (1) obtain reference curve from market context; (2) compute PV of remaining cashflows at reference rate + make-whole spread; (3) set `call_price = max(outstanding * price_pct_of_par / 100, pv_remaining)`. Requires passing `MakeWholeSpec` and reference curve into the tree state. See `convertible/README.md` for similar planned work.

---

### 1.4 CMS swap forward rate shared utility

**Location:** `valuations/src/instruments/rates/cms_swap/pricer.rs:250`

**Current limitation:** `CmsOptionPricer::calculate_forward_swap_rate` requires a `&CmsOption`; CMS swap pricing builds a proxy `CmsOption` just to call it, coupling CMS swap to CMS option types.

**Remediation:** Extract `calculate_forward_swap_rate(discount_curve_id, forward_curve_id, swap_convention, fixed_freq, float_freq, day_counts, as_of, swap_start, swap_end)` into a shared module (e.g. `instruments/rates/shared/forward_swap_rate.rs`). Both `CmsOptionPricer` and `CmsSwapPricer` call this function with their respective parameters. Remove `build_cms_option_proxy()`.

---

### 1.5 Student-t calibration full implementation

**Location:** `valuations/src/calibration/targets/student_t.rs:104-118`

**Current limitation:** `StudentTCalibrator::solve()` returns `Err`; `calibrate_df()` uses a placeholder objective `df - initial_df` instead of tranche repricing. No tranche lookup, no `StudentTCopula` construction, no model vs market upfront comparison.

**Remediation:** (1) Add `tranche_instrument_id` / quote lookup to resolve tranche and market upfront from `MarketQuote`. (2) For each trial `df`, build `StudentTCopula::new(df)`, inject into market context, price tranche via existing CDO tranche pricer. (3) Set `objective(df) = (model_upfront - market_upfront) / notional`. (4) Wire `StudentTCalibrator` into `execute_params` so the step is accepted when quotes and tranche are available.

---

### 1.6 Equity price scalar attribution

**Location:** `valuations/tests/attribution/scalars_attribution.rs:12`

**Current limitation:** `ScalarsSnapshot::extract` and `restore_scalars` exist, but equity instruments do not use `price_id` consistently. Attribution cannot detect spot price changes for equity options/forwards.

**Remediation:** Ensure `Equity`, `EquityOption`, `EquityForward` (and related) store and use `price_id` (e.g. `AAPL-SPOT`, `EQUITY-SPOT`) for spot lookup. Update pricers to call `market.price(price_id)` and include that scalar in attribution. Add `MarketExtractable` impl for equity price scalars and enable the test.

---

### 1.7 Inflation swap bucketed DV01

**Location:** `valuations/tests/instruments/inflation_swap/metrics/test_bucketed_dv01.rs:88`

**Current limitation:** Inflation swap uses `UnifiedDv01Calculator` with `triangular_key_rate()`. Historically, bucketed DV01 returned 0 because the pricer did not respond to discount curve bumps. Test comment indicates a known limitation; assertions were relaxed or updated.

**Remediation:** Verify that `InflationSwap` pricer uses discount curve from `MarketContext` for both legs and that `UnifiedDv01Calculator` bump-and-reprice hits the correct curve. If the inflation leg is insensitive to discount bumps (e.g. uses a different curve or formula), ensure the fixed leg’s discount sensitivity is captured. Add an integration test that bumps the discount curve by 1bp and asserts `|ΔPV| ≈ bucketed_dv01 * 0.0001` within tolerance.

---

### 1.8 Waterfall diversion tracking

**Location:** `valuations/tests/instruments/structured_credit/waterfall_golden.rs:413`

**Current limitation:** `had_diversions` is set when OC/IC tests fail, but `diverted_cash` tracking is limited to tiers that actually redirect. Not all diverted amounts (e.g. interest diverted to OC cure) are captured in a structured way.

**Remediation:** Extend `AllocationOutput` or `ExplanationTrace` with `diverted_amounts: Vec<(tranche_id, amount, reason)>`. In the waterfall, when a diversion occurs, record the amount and source (e.g. interest from tier X diverted to OC cure for tranche Y). Update golden tests to assert on `diverted_amounts` when `had_diversions` is true.

---

### 1.9 Discrete dividend modeling (equity)

**Location:** `valuations/tests/instruments/equity/pricer_tests.rs:339`

**Current limitation:** Forward/option pricing uses continuous yield `q` via `exp((r-q)t)`. Market standard for single-name equities is discrete cash dividends: `F = S*exp(r*t) - Σ D_i*exp(r*(t-t_i))`.

**Remediation:** Add `discrete_dividends: Vec<(Date, f64)>` to `Equity` (and optionally `EquityForward`). Update `forward_price_per_share()` to subtract discounted future dividends. Ensure `EquityOption` pricer (Black-Scholes, tree) uses the adjusted forward. Add validation against market data (e.g. single-name equity forward with known div schedule).

---

## 2. Unimplemented / Not Yet Implemented

### 2.1 Student-t calibration step (entry point)

**Location:** `valuations/src/calibration/targets/student_t.rs:90`

**Current limitation:** `StudentTCalibrator::solve()` always returns `Err("tranche repricing is not yet wired")`. Calibration pipeline rejects the step.

**Remediation:** Same as 1.5 – implement tranche repricing and wire into `solve()`. Until then, document that Student-t calibration is experimental and keep the explicit `Err` to avoid silent failure.

---

### 2.2 SVI surface calibration

**Location:** `valuations/src/calibration/step_runtime.rs:250`, `validation/preflight.rs:62`

**Current limitation:** SVI calibration step returns `Err("quote extraction not yet wired")`. `calibrate_svi()` exists in finstack-core but vol quotes are not extracted from `MarketQuote::Vol`, grouped by expiry, or passed through.

**Remediation:** In `execute_params` for `StepParams::SviSurface`: (1) filter `quotes` for `MarketQuote::Vol(VolQuote::OptionVol { .. })`; (2) group by expiry; (3) build `(strikes, vols)` per expiry; (4) call `finstack_core::calibrate_svi` per slice; (5) construct `VolSurface` from SVI parameters and insert into context.

---

### 2.3 Bermudan swaption value

**Location:** `valuations/src/instruments/rates/swaption/metrics/mod.rs:60`

**Current limitation:** `BermudanSwaption::value()` is not implemented; it requires a tree or LSMC pricer. Only European swaptions have closed-form value.

**Remediation:** Implement `value()` by delegating to `TreeValuator` or `SwaptionLsmcPricer` when `bermudan_type != CoTerminal` is not required. For co-terminal Bermudans, use the existing tree. Document that non-co-terminal Bermudans are unsupported (see 2.5).

---

### 2.4 Non-co-terminal Bermudan swaptions

**Location:** `valuations/src/instruments/rates/swaption/pricing/tree_valuator.rs:110-114`

**Current limitation:** Tree valuator rejects `BermudanType::NonCoTerminal` because each exercise date would need a different swap end date, which is not implemented.

**Remediation:** Extend tree/LSMC to support per-exercise swap end dates. Store `swap_end_dates: Vec<Date>` (one per exercise) and use the appropriate end date when valuing the underlying swap at each exercise. Requires schedule logic for non-co-terminal structures.

---

### 2.5 Commodity Bermudan options

**Location:** `valuations/src/instruments/commodity/commodity_option/types.rs:596`

**Current limitation:** Bermudan exercise returns `Err`; American approximation was removed because it overstates value.

**Remediation:** Implement Bermudan pricing via (1) binomial/trinomial tree with early exercise at each date, or (2) LSMC. Reuse patterns from `SwaptionLsmcPricer` or equity American option tree. Add `exercise_dates: Vec<Date>` to the instrument and wire into the pricer.

---

### 2.6 Seasoned Geometric Asian options

**Location:** `valuations/src/instruments/exotics/asian_option/pricer.rs:842`

**Current limitation:** When `count > 0` fixings are observed, analytical pricer returns `Err`. Effective strike formula `K_eff = (n·K - G_past) / (n - m)` is not implemented.

**Remediation:** Compute `G_past` from observed fixings, then `K_eff = (n * K - G_past) / (n - m)`. Use `geometric_asian_call(spot, K_eff, t_remaining, r, q, sigma, n - m)` with adjusted number of future fixings. Add tests for partially seasoned paths.

---

### 2.7 FX American/Bermudan options

**Location:** `valuations/src/instruments/fx/fx_option/calculator.rs:385`

**Current limitation:** Only European exercise is supported. American and Bermudan return `Err("specialized pricers not yet implemented")`.

**Remediation:** Add tree pricer for American FX options (reuse equity tree pattern with Garman-Kohlhagen). For Bermudan, add `exercise_dates` and tree/LSMC with early exercise at those dates. Register pricer by `ExerciseStyle`.

---

### 2.8 Swaption payoff swap value placeholder

**Location:** `valuations/src/instruments/common/models/monte_carlo/payoff/swaption.rs:174-182`

**Current limitation:** `compute_swap_value_from_rate` returns `short_rate - strike` as a placeholder. Real implementation needs Hull-White bond prices and annuity.

**Remediation:** Implement in `SwaptionLsmcPricer`: compute `P(t,T)` and `A(t)` from HW parameters, then `S(t) = [P(t,T_0) - P(t,T_N)] / A(t)`. Call `set_swap_value` on the payoff before `on_event`. Remove the placeholder formula from the payoff struct.

---

### 2.9 Statements: rounding context

**Location:** `statements/src/evaluator/engine.rs:393`

**Current limitation:** `ResultsMeta.rounding_context` is always `None`. Rounding rules (e.g. to 2 decimals for reporting) are not applied.

**Remediation:** Add `RoundingContext { decimals: u32, mode: RoundingMode }` to evaluation config. After computing raw values, apply rounding when populating `StatementResult`. Document rounding behavior in API docs.

---

### 2.10 Statements: Monte Carlo + capital structure

**Location:** `statements/src/evaluator/engine.rs:438`

**Current limitation:** `evaluate_monte_carlo` returns `Err` when `model.capital_structure.is_some()`. Capital structure (cs.*) references cannot be used in MC mode.

**Remediation:** Either (1) support capital structure in MC by evaluating instruments per path and aggregating, or (2) document that MC is for forecast-only models and direct users to `finstack-valuations` for capital structure MC. Option (1) requires instrument pricing per path and dependency resolution for cs.* nodes.

---

### 2.11 Statements: formula adjustments

**Location:** `statements/src/adjustments/engine.rs:107`

**Current limitation:** `AdjustmentValue::Formula` returns `Err("Formula adjustments not yet implemented")`. Formula-based caps/floors (e.g. 20% of EBITDA) cannot be used.

**Remediation:** Reuse the formula compiler from the main evaluator. Resolve identifiers (e.g. `ebitda`) from `results.nodes`, evaluate the formula, and use the result as the cap/floor value. Add tests for formula-based adjustments.

---

### 2.12 Statements: full grid sensitivity

**Location:** `statements/src/analysis/sensitivity.rs:107`

**Current limitation:** `run_full_grid` returns `Err("Full grid sensitivity not yet implemented")`. Only diagonal and tornado are supported.

**Remediation:** Implement factorial design: for each parameter, create ±1σ (or configurable) scenarios; take Cartesian product of all parameter combinations. Run model for each combination and build a sensitivity matrix. Consider limiting to small grids (e.g. max 5 params) to avoid explosion.

---

### 2.13 Scenarios: BaseCorr maturities filter

**Location:** `scenarios/src/adapters/basecorr.rs:57`

**Current limitation:** `BaseCorrBucketPts` with `maturities` filter emits a warning and ignores maturities; only detachment-based bump is applied.

**Remediation:** Filter base correlation surface points by maturity before applying the bump. If `maturities` is non-empty, only bump detachments at those maturities; otherwise keep current behavior. Update `MarketBump::BaseCorrBucketPts` handling in the scenarios engine.

---

### 2.14 Scenarios: BaseCorr test context hack

**Location:** `scenarios/src/adapters/basecorr.rs:104`

**Current limitation:** Comment: "Hack: generic context not needed?" – unit test cannot easily construct `ExecutionContext` for `BaseCorrAdapter::try_generate_effects`.

**Remediation:** Add `ExecutionContext::test_context()` or `ExecutionContext::minimal()` for tests. Alternatively, move the integration test to a crate that can build a real context. Remove the hack comment once tests are properly structured.

---

### 2.15 Portfolio margin: cross-currency

**Location:** `portfolio/src/margin/README.md:252`

**Current limitation:** Margin aggregation assumes single base currency. Cross-currency FX conversion for margin is not implemented.

**Remediation:** Add FX rate lookup from `MarketContext` (or `FinstackConfig`). When aggregating margin across positions, convert each position’s margin to base currency using spot (or configurable) FX before summing. Document FX convention (e.g. as-of date, fixing source).

---

### 2.16 Portfolio margin: full ISDA SIMM

**Location:** `portfolio/src/margin/README.md:254`

**Current limitation:** SIMM calculator uses simplified correlation handling. Full ISDA SIMM v2.6 cross-bucket and cross-risk-class correlations are not implemented.

**Remediation:** Implement full SIMM correlation matrix per ISDA SIMM v2.6. Add bucket/risk-class mapping and correlation lookup. This is a larger project; consider incremental delivery (e.g. rates + credit first, then FX, equity, commodity).

---

### 2.17 XVA: wrong-way risk

**Location:** `valuations/src/xva/types.rs:72`

**Current limitation:** `include_wrong_way_risk` is a placeholder; correlation between exposure and default probability is not modeled.

**Remediation:** When enabled, use a copula or correlation parameter to link exposure paths to default intensity. For example, scale hazard rate by exposure level (e.g. `λ(t) = λ_base * (1 + α * E(t)/E_avg)`). Requires exposure profile and calibration of α. Document as experimental.

---

### 2.18 XVA: MPOR in exposure engine

**Location:** `valuations/src/xva/types.rs:525`

**Current limitation:** `mpor_days` is stored but not used. Deterministic exposure engine does not model gap risk during the close-out period.

**Remediation:** In exposure simulation, when default occurs at t, hold exposure at `E(t)` for `mpor_days` (or equivalent in time steps) before applying collateral. Uncollateralized exposure = `max(E(t), E(t+MPOR))` or similar, per regulatory treatment. Update CVA formula to use MPOR-adjusted exposure.

---

### 2.19 Calibration: efficient Jacobian

**Location:** `valuations/src/calibration/solver/traits.rs:178`

**Current limitation:** Some targets return `Err("Efficient Jacobian not implemented")` when analytical derivatives are requested.

**Remediation:** For each target type, either (1) implement `jacobian()` analytically, or (2) fall back to finite-difference Jacobian and document. Finite-difference is a valid fallback; ensure it is used when analytical is unavailable rather than failing.

---

### 2.20 Benchmark: t-distribution inverse CDF

**Location:** `core/src/analytics/benchmark.rs:244`

**Current limitation:** Beta confidence interval uses 1.96 (normal quantile) instead of t-quantile. For small n, CI is slightly too narrow.

**Remediation:** Add `inv_cdf_t(nu: f64, p: f64)` using beta function or `statrs::distribution::StudentsT`. Use `t_quantile(n-2, 0.975)` instead of 1.96 for BetaResult CI. Low priority; document as acceptable for n > 40.

---

## 3. Placeholders and Stubs

### 3.1 Student-t calibrator placeholder objective

**Location:** `valuations/src/calibration/targets/student_t.rs:111-124`

**Current limitation:** `calibrate_df()` uses `objective(df) = df - initial_df` so the solver converges on the initial guess. No real calibration.

**Remediation:** Same as 1.5 – replace with tranche repricing residual. The `calibrate_df` path is currently dead because `solve()` returns `Err`; once `solve()` is implemented, `calibrate_df` should use the real objective.

---

### 3.2 Discount calibration Deposit placeholder

**Location:** `valuations/src/calibration/targets/discount.rs:1355`

**Current limitation:** When building `initial_guess`, a `Deposit` instrument is used as a placeholder. May not match the actual calibration target.

**Remediation:** Use the first calibration quote’s instrument type (e.g. FRA, swap) to build the initial guess, or accept a generic `Instrument` in the bootstrap config. Ensure the placeholder has the correct maturity and rate for the first pillar.

---

### 3.3 Swaption LSMC placeholder values

**Location:** `valuations/src/instruments/common/models/monte_carlo/pricer/swaption_lsmc.rs:299-302`

**Current limitation:** Regression outputs `out[degree+1] = 1.0` (placeholder for A) and `out[degree+2] = swap_rate` (placeholder for S×A). LSMC continuation value may be inaccurate.

**Remediation:** Compute annuity `A(t)` and forward swap rate `S(t)` from Hull-White at each exercise date. Use `A` and `S` (or `S*A`) as regression features. Implement as in 2.8.

---

### 3.4 Prepayment spec placeholder

**Location:** `valuations/src/instruments/fixed_income/structured_credit/pricing/stochastic/prepayment/spec.rs:199`

**Current limitation:** Some prepayment spec resolution returns `None` as placeholder.

**Remediation:** Identify the missing case (e.g. pool-level CPR from deal docs) and implement proper lookup. If the feature is intentionally optional, document and ensure callers handle `None`.

---

### 3.5 CDS tranche credit index placeholder

**Location:** `valuations/src/instruments/credit_derivatives/cds_tranche/types.rs:83`

**Current limitation:** Credit index identifier for survival/loss modeling is a placeholder.

**Remediation:** Wire to `base_correlation_curve_id` or a dedicated `credit_index_id` from the instrument. Use it for index-level default/survival in tranche pricing. Ensure it is validated at construction.

---

### 3.6 Generic asset placeholder

**Location:** `valuations/src/instruments/fixed_income/structured_credit/types/enums.rs:224`

**Current limitation:** `Generic` asset type is a placeholder for non-standard assets.

**Remediation:** Either (1) implement generic asset behavior (e.g. configurable default curve, recovery), or (2) document as reserved for future use and reject in strict validation. Prefer (1) for flexibility in CLO/ABS deals.

---

### 3.7 Equity spot pricer curves placeholder

**Location:** `valuations/src/instruments/equity/spot/pricer.rs:43`

**Current limitation:** `curves` parameter is unused; documented as "placeholder for quotes".

**Remediation:** Use `curves` (or `market`) to look up `price_id` for spot. If spot is in `MarketScalar::Price`, use it. Remove "unused" and document the intended flow for quote-driven pricing.

---

### 3.8 Cap/floor and CDS option ImpliedVol placeholder

**Location:** `valuations/src/instruments/rates/cap_floor/metrics/mod.rs:13`, `cds_option/metrics/mod.rs:10`

**Current limitation:** ImpliedVol metric is listed as placeholder; not implemented.

**Remediation:** Implement implied vol calculator: given market price, solve for σ in Black/Cap formula using Brent or Newton. Register the calculator in the metrics module. Add tests against known vol/price pairs.

---

### 3.9 Bond tree volatility placeholder

**Location:** `valuations/src/instruments/fixed_income/bond/pricing/engine/tree.rs:574`

**Current limitation:** `volatility: 0.01` is a placeholder; overridden by calibrated sigma.

**Remediation:** Ensure calibration always runs before tree build, or provide a clear default (e.g. from bond option vol surface). If no calibration, document that 0.01 is used and may be inaccurate. Consider `Option<f64>` to force explicit choice.

---

### 3.10 FX swap near leg placeholder

**Location:** `valuations/src/instruments/fx/fx_swap/pricing_helper.rs:95`

**Current limitation:** When near leg has settled and `include_near` is false, uses `1.0` as placeholder for the near leg DF.

**Remediation:** Use actual settlement date and DF to settlement for consistency. If the near leg is excluded from PV, document that the 1.0 is intentional (e.g. "near leg notional already settled") and ensure it does not affect far leg valuation.

---

### 3.11 Commodity forward spot fallback

**Location:** `valuations/src/instruments/commodity/commodity_forward/types.rs:320`

**Current limitation:** When no discount curve is available, returns spot as approximation. No discounting.

**Remediation:** Require discount curve for commodity forwards, or document that spot-only is a simplified mode. If spot fallback is kept, add a warning log and document in the type’s docstring.

---

## 4. Hacks and Workarounds

### 4.1 BaseCorr test context

**Location:** `scenarios/src/adapters/basecorr.rs:104`

**Current limitation:** Unit test cannot construct `ExecutionContext`; hack comment left in place.

**Remediation:** Add `ExecutionContext::for_test()` or use integration test with real context. See 2.14.

---

### 4.2 Tolerance hack in tests

**Location:** `valuations/tests/instruments/README.md:250`

**Current limitation:** Documented anti-pattern: "Tolerance hack that masks a bug". Some tests use loose tolerances instead of fixing the root cause.

**Remediation:** Audit tests that use `assert_approx_eq` with tolerance > 1e-6. Identify the underlying numerical or model issue, fix it, and tighten tolerance. Document expected accuracy in test comments.

---

## 5. Summary by Priority

| Priority | Category | Count | Notes |
|----------|----------|-------|------|
| High | Calibration (Student-t, SVI) | 2 | Blocks structured credit workflows |
| High | Make-whole, Bermudan support | 3 | Market-standard features |
| Medium | Attribution, WAL, diversion | 4 | Improves analytics quality |
| Medium | Statements (formula, MC, sensitivity) | 4 | Extends statement capabilities |
| Medium | Discrete dividends, inflation coupons | 2 | Market-standard equity/inflation |
| Low | Placeholders, XVA, benchmark | 10+ | Incremental improvements |

---

*Generated from codebase investigation. Update this document as items are completed.*
