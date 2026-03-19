# Finstack Comprehensive Audit Fix Plan

## Context

A systematic file-by-file audit of all 14 Rust crates (~2,500 source files) identified 188 findings: 8 CRITICAL, 22 HIGH, 62 MEDIUM, 96 LOW. No core pricing formula errors (Black-Scholes, SABR, Heston, etc.), but significant issues in error handling, numerical edge cases, scenario engine robustness, and P&L attribution. This plan organizes all fixes into 6 dependency-ordered phases.

---

## Phase 1: CRITICAL Fixes (8 items)

### C1: MC Barrier dt for non-uniform grids
**File:** `finstack/monte_carlo/src/payoff/barrier.rs`
- Remove single `dt` field from `BarrierOptionPayoff`
- Store `step_dts: Vec<f64>` precomputed from `TimeGrid` at construction
- In `on_event()`, use `self.step_dts[state.step - 1]` instead of `self.dt`
- `gobet_miri_adjusted_barrier()` in `corrections.rs` already accepts `dt: f64` — no changes needed
- Update all callers of `BarrierOptionPayoff::new` (search workspace)
- **Test:** Non-uniform TimeGrid confirming different dt per step; regression test with uniform grid

### C2: MC Antithetic bypasses populate_path_state()
**File:** `finstack/monte_carlo/src/variance_reduction/antithetic.rs:155-183`
- Replace manual `set(SPOT, ...)` / `set(VARIANCE, ...)` with `process.populate_path_state(state, &mut path_state)`
- Match pattern from `engine.rs:1370-1374`
- Apply to both initial state (line 155-159) and per-step update (lines 175-187)
- **Test:** Mock process with 3+ state dimensions; regression test for GBM antithetic pricing

### C3: Portfolio Attribution FX flow_translation not in total_pnl
**File:** `finstack/portfolio/src/attribution.rs:471`
- Line 471 pushes only `principal_translation` to `total_pnl_vals`
- Change to push `total_translation` (= flow + principal) to match documented decomposition `total = factors + fx_translation + residual`
- **Test:** Cross-currency attribution with known FX rates verifying P&L identity reconciles

### C4: Portfolio Margin silently swallows sensitivity errors
**Files:** `finstack/portfolio/src/margin/aggregator.rs:101-107`, `finstack/portfolio/src/error.rs`
- Change `if let Ok(sensitivities)` to `match` with error accumulation
- Add `degraded_positions: Vec<(PositionId, String)>` to `PortfolioMarginResult`
- Add `tracing::warn!` per failed position
- Apply same pattern to `get_position_mtm` at line 164
- **Test:** Position returning error from `simm_sensitivities()` populates degraded list

### C5: Scenario curve resolution currency-prefix heuristic
**File:** `finstack/scenarios/src/adapters/curves.rs:144-175`
- Add optional explicit `discount_curve_id` parameter to operations needing it
- When heuristic matches, validate curve exists in market context
- Return `Error` instead of `None` / hardcoded "USD-OIS" fallback (line 285)
- Log warning when using heuristic fallback
- **Test:** Explicit curve ID bypass; ambiguous prefix with two matching curves

### C6: Scenario apply_forecast_assign overwrites all periods
**File:** `finstack/scenarios/src/adapters/statements.rs:87-95`
- Add `apply_forecast_assign_filtered(model, node_id, value, period_filter: Option<(Date, Date)>)`
- Filter entries by period key within range when `period_filter` is `Some`
- Keep existing `apply_forecast_assign()` as wrapper with `None`
- **Test:** 4-period model, assign to only period 2, verify others unchanged

### C7: Scenario time roll single failure aborts all
**File:** `finstack/scenarios/src/adapters/time_roll.rs:219`
- Replace `?` on `checked_sub` with per-instrument `match`
- Collect failures in `Vec<(String, String)>`, add to `RollForwardReport`
- Continue processing remaining instruments after one fails
- **Test:** Instrument with currency mismatch doesn't abort remaining instruments

### C8: Analytics CAGR Act/365 Fixed bias
**File:** `finstack/analytics/src/risk_metrics/return_based.rs:56`
- Add `AnnualizationConvention` enum: `Act365Fixed`, `Act365_25`, `ActAct`
- Add `cagr_with_convention()` function
- Change `cagr()` default from `365.0` to `365.25`
- Update downstream: Calmar, Sterling, Martin ratios
- **Test:** CAGR over leap-year period; `Act365Fixed` matches old behavior

---

## Phase 2: HIGH Fixes — Foundation Crates (10 items)

### finstack-core (6)

| # | Finding | File | Fix |
|---|---------|------|-----|
| H1 | Dupire local vol missing r/q | `math/volatility/local_vol.rs:213` | Accept rate curves, include drift terms; or document forward-measure assumption |
| H2 | Brownian bridge inconsistency | `math/random/brownian_bridge.rs:79` | Add test asserting `construct_path` and `construct_path_irregular` produce same distribution on uniform grid |
| H3 | Realized variance naming | `math/stats.rs:298` | Rename to `realized_second_moment` or add prominent doc note |
| H4 | Forward rate one-sided diff | `market_data/traits.rs:214` | Use centered difference `(f(t+eps) - f(t-eps)) / (2*eps)`, cap eps at 1e-4 |
| H5 | Vol surface interp on vol not variance | `market_data/surfaces/vol_surface.rs:185` | Add `VolInterpolationMode` enum (Vol, TotalVariance); default to Vol for backward compat |
| H6 | FX delta vol 10-delta unused | `market_data/surfaces/delta_vol_surface.rs:299` | Add `rr_10d`/`bf_10d` optional fields to builder; 5-point smile when available |

### finstack-cashflows (3)

| # | Finding | File | Fix |
|---|---------|------|-----|
| H7 | `unwrap_or(Decimal::ZERO)` x4 | `builder.rs:864,1085,1103,1187` | Replace with `map_err(...)? ` returning proper errors |

### finstack-correlation (1)

| # | Finding | File | Fix |
|---|---------|------|-----|
| H8 | Student-t copula wrong df | `copula/student_t.rs:235-237` | Change `student_t_cdf(threshold_adj, nu)` to `student_t_cdf(threshold_adj, nu + 1)` |

---

## Phase 3: HIGH Fixes — Numerical Engines (8 items)

### finstack-analytics (3)

| # | Finding | File | Fix |
|---|---------|------|-----|
| H9 | CF-VaR sign in modified Sharpe | `risk_metrics/tail_risk.rs:560` | Handle positive CF-VaR edge case |
| H10 | Rolling kernel float drift | `risk_metrics/rolling.rs:248` | Add periodic recalculation every ~1000 steps |
| H11 | Beta CI z vs t | `benchmark.rs:345` | Add t-distribution option for n < 40 |

### finstack-margin (2)

| # | Finding | File | Fix |
|---|---------|------|-----|
| H12 | SIMM curvature simplified | `calculators/im/simm.rs:364` | Implement cross-bucket correlation aggregation per ISDA spec |
| H13 | SIMM commodity delta no correlation | `calculators/im/simm.rs:306` | Add intra/inter-bucket correlation matrices |

### finstack-monte-carlo (3 additional HIGH)

| # | Finding | File | Fix |
|---|---------|------|-----|
| H-MC1 | Milstein sigma' only valid for GBM | `discretization/milstein.rs:91` | Add runtime check or trait bound ensuring GBM-like processes |
| H-MC2 | QE Heston spot update safeguard | `discretization/qe_heston.rs:300` | Tighter sigma_v guard, check int_var >= 0 before sqrt |
| H-MC3 | LSMC ITM threshold | `pricer/lsmc.rs:298` | Use `immediate > 0.0` instead of `> 1e-6` |

---

## Phase 4: HIGH Fixes — Application Crates (9 items)

### finstack-valuations (3)

| # | Finding | File | Fix |
|---|---------|------|-----|
| H14 | CMS convexity first-order | `instruments/rates/cms_option/pricer.rs:296` | Document; ensure replication pricer is default for long-dated CMS |
| H15 | Bermudan swaption defaults | `instruments/rates/swaption/pricer.rs:380` | Log warning when uncalibrated `HullWhiteParams::default()` used |
| H16 | Variance swap ATM fallback | `instruments/equity/variance_swap/pricer.rs:364` | Add convexity correction from smile curvature |

### finstack-portfolio (4)

| # | Finding | File | Fix |
|---|---------|------|-----|
| H17 | FX 1:1 fallback in margin | `margin/aggregator.rs:232` | Return error instead of silently using 1:1 rate |
| H18 | Cashflow FX spot for far-future | `cashflows.rs:160` | Warn when date > as_of + 30Y |
| H19 | Metrics currency ambiguity | `metrics.rs:336` | Add currency field to per-position metrics |
| H20 | UnitScaling dimensional mismatch | `optimization/decision.rs:37` | Document or fix PV-based weight vs unit-based quantity |

### finstack-scenarios (5)

| # | Finding | File | Fix |
|---|---------|------|-----|
| H21 | FX no cross-rate consistency | `adapters/fx.rs` | Add post-shock triangulation check with warning |
| H22 | VolIndex bp inconsistency | `adapters/curves.rs:323` | Normalize and document bp semantics |
| H23 | Instrument shocks last-wins | `adapters/instruments.rs:74` | Accumulate: `old_pct + new_pct` instead of overwrite |
| H24 | Time roll MV always empty | `adapters/time_roll.rs:208` | Remove dead fields from report struct |
| H25 | Macros non-hygienic paths | `valuations/macros/src/instrument.rs:62` | Document internal-only constraint |

---

## Phase 5: MEDIUM Fixes (62 items)

### Sprint 5A: finstack-core (16 items)
- LM solver probe fragile (`solver_multi.rs:723`) — require `residual_count()` implementation
- SABR cross-zero fallback (`sabr.rs:262`) — improve midpoint approximation
- Heston integration upper bound (`heston.rs:243`) — add second pass when suspect
- `validate_correlation_matrix` assert -> Result (`linalg.rs:657`)
- Gauss-Hermite convention documentation (`integration.rs:333`)
- OnlineCovariance merge formula (`stats.rs:686`) — use delta-based formula
- SobolRng panicking constructor (`random/sobol.rs:131`) — deprecate `new()`, prefer `try_new()`
- Hazard curve bump clamp warning (`market_data/bumps.rs:487`)
- Inflation key-rate bump crude (`context/curve_storage.rs:238`) — triangular weighting
- `bump_observed` clones entire context (`context/ops_bump.rs:209`) — document prefer in-place
- ACT/ACT ISMA unbounded recursion (`dates/daycount.rs:722`) — add depth guard
- XIRR rejects multi-sign-change (`cashflow/xirr.rs:273`) — demote to warning
- FxMatrix single-pivot triangulation (`money/fx.rs:803`) — document limitation
- Rate::as_bps i32 overflow (`types/rates.rs:230`) — document or use i64
- DAG cycle detection recursive DFS (`expr/dag.rs:309`) — add depth limit
- MarketContext Send+Sync assertion (`market_data/context/mod.rs:32`)

### Sprint 5B: finstack-cashflows + correlation + monte-carlo (14 items)
- Swallowed day count errors (`cashflows/dataframe.rs`)
- SMM-to-CPR missing release validation
- Gaussian copula correlation gap 1e-10 vs 0.01 (`copula/gaussian.rs:132`)
- Low-order quadrature in Student-t (`copula/student_t.rs:161`)
- Gamma variate flooring (`copula/student_t.rs:215`)
- Multi-factor quadrature 5 points/dim (`copula/multi_factor.rs:46`)
- RFL tail dependence heuristic (`copula/random_factor_loading.rs:188`)
- Correlated recovery clamping bias (`recovery/correlated.rs:143`)
- MC seed DefaultHasher not stable (`seed.rs:25`)
- MC captured path percentile approximate (`captured_path_stats.rs:21`)
- Exact GBM dividend shock splitting bias (`discretization/exact_gbm_dividends.rs:106`)
- PathState::vars() allocates HashMap (`traits.rs:312`)
- PhiloxRng wastes normal for odd buffer (`rng/philox.rs:228`)
- state_keys::indexed_spot leaks memory (`traits.rs:100`)

### Sprint 5C: finstack-analytics + margin (12 items)
- `moments4` one-pass vs two-pass inconsistency
- `mean_return` arithmetic annualization documentation
- Beta CI z=1.96 documentation
- Schedule IM hardcoded grid
- CSA threshold application
- VM bilateral netting
- Various SIMM parameter validations

### Sprint 5D: finstack-valuations (13 items)
- Spread metric list hardcoded (`pricer/registry.rs:238`)
- HW1F forward rate fallback returns 3% (`calibration/hull_white.rs:408`)
- HW1F initial guess hardcoded (`calibration/hull_white.rs:269`)
- Taylor attribution only discount curves (`attribution/taylor.rs:145`)
- Currency inference fragile string matching (`calibration/bumps/rates.rs:22`)
- Cap/floor auto-fallback between Black/Bachelier
- Structured credit overflow in `to_currency_units`
- Equity discrete dividend American limitation documentation
- Various calibration and metric edge cases

### Sprint 5E: finstack-statements + portfolio + scenarios (13 items)
- Statements: MC correlation, forecast edge cases, defensive coding
- Portfolio: PvNative/PvBase mismatch, book cycle comment, grouping recursion depth
- Scenarios: empty attrs matches all, compose loses names, unhandled ops warn not error, FX pct floor validation, vol arbitrage check, attribute matching only searches meta map

---

## Phase 6: LOW Fixes (96 items)

### Sprint 6A: Documentation/naming (25 items)
Rename misleading functions, add missing docstrings, clarify conventions

### Sprint 6B: Debug assertions -> runtime checks (10 items)
Promote `debug_assert!` to runtime checks where failure would produce wrong results silently

### Sprint 6C: Minor edge cases (20 items)
Handle degenerate inputs (empty slices, NaN, infinity, single-element collections)

### Sprint 6D: Minor performance (15 items)
Reduce allocations in hot paths, avoid redundant computation

### Sprint 6E: API consistency (15 items)
Align similar APIs, standardize error messages, add `#[must_use]`

### Sprint 6F: Minor correctness (11 items)
Floating-point accumulation, rounding, clamping consistency

---

## Verification Strategy

Each phase must pass before proceeding:
1. `cargo test --workspace` — all existing tests pass
2. `cargo clippy --workspace` — no new warnings
3. New tests added for each fix (especially CRITICAL/HIGH)
4. Phase 1 CRITICALs: manual verification against known analytical solutions
5. Phase 2-3 numerical fixes: regression benchmarks confirming values match or improve
6. Phase 4-6: standard unit test coverage

### High-Risk Changes Requiring Extra Review
- **C3** (attribution FX): Verify P&L identity with real portfolio data before/after
- **C1** (barrier dt): API-breaking constructor change — search all callers
- **C8** (CAGR 365->365.25): Changes numerical output — coordinate with reporting
- **H12/H13** (SIMM): Verify against ISDA CRIF test pack if available
