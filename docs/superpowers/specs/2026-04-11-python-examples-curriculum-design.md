# Python Examples Curriculum Design Spec

## Goal

Create a layered Jupyter notebook curriculum under `finstack-py/examples/notebooks/` that teaches a starting quant how to use the full finstack Python library. Replaces the legacy examples in `.audit/legacy-archive/python-examples/`.

## Format

Jupyter notebooks (`.ipynb`). Each notebook follows a consistent structure:
1. **Title cell** — markdown with notebook name, one-line purpose, prerequisites
2. **Concept introduction** — brief markdown explaining the financial/quant concept
3. **API walkthrough** — code cells demonstrating each class/function with inline commentary
4. **Realistic mini-example** — a complete worked scenario tying the concepts together
5. **Key takeaways** — markdown summary of what was covered and cross-references

## Organization

Two tiers:
- **Numbered curriculum notebooks (01-16)**: The linear learning path. A quant reads these in order.
- **Deep-dive sub-directories**: Reference notebooks for specific topics (instruments, curves, models, etc.). Jump to these as needed.

## Design Principles

- **No stubs**: Every notebook ships complete or not at all
- **Self-contained data**: Each notebook creates sample data inline — no external CSV dependencies
- **Progressive imports**: Early notebooks import only what they need; later ones import broadly
- **Polars for display**: Use `to_polars_*` where available for clean tabular output
- **Cross-references**: Notebooks link back to prerequisites
- **Consistent notebook pattern**: Title → Concept → API walkthrough → Mini-example → Takeaways

## External Dependencies

- `pandas` — required by `finstack.analytics.Performance`
- `polars` — for `to_polars_long` / `to_polars_wide` exports from statements
- No matplotlib/plotting required (notebooks focus on computation, not visualization)

---

## Directory Layout

```
finstack-py/examples/notebooks/
├── README.md
├── run_all_notebooks.py
│
├── 01_foundations/
│   ├── 01_core_types_and_money.ipynb
│   ├── 02_dates_calendars_schedules.ipynb
│   ├── 03_market_data_and_curves.ipynb
│   ├── 04_math_toolkit.ipynb
│   ├── market_data/
│   │   ├── discount_curves.ipynb
│   │   ├── forward_curves.ipynb
│   │   ├── hazard_curves.ipynb
│   │   ├── price_curves.ipynb
│   │   ├── volatility_index_curves.ipynb
│   │   ├── vol_surfaces.ipynb
│   │   ├── fx_matrix.ipynb
│   │   └── inflation_curves.ipynb
│   └── dates/
│       ├── day_count_conventions.ipynb
│       ├── holiday_calendars.ipynb
│       └── schedule_building.ipynb
│
├── 02_pricing/
│   ├── 05_pricing_fundamentals.ipynb
│   ├── 06_pricing_across_asset_classes.ipynb
│   └── instruments/
│       ├── complex_cashflows.ipynb
│       ├── bonds_and_fixed_income.ipynb
│       ├── loans_and_credit_facilities.ipynb
│       ├── rates_derivatives.ipynb
│       ├── credit_derivatives.ipynb
│       ├── equity_and_options.ipynb
│       ├── fx_instruments.ipynb
│       ├── inflation_linked.ipynb
│       ├── structured_credit.ipynb
│       ├── convertible_bonds.ipynb
│       ├── repo_and_financing.ipynb
│       └── total_return_and_variance_swaps.ipynb
│
├── 03_analytics/
│   ├── 07_performance_analytics.ipynb
│   └── 08_risk_and_factor_analytics.ipynb
│
├── 04_statement_modeling/
│   ├── 09_statement_modeling.ipynb
│   ├── 10_statement_analytics.ipynb
│   └── models/
│       ├── three_statement_model.ipynb
│       ├── lbo_analysis.ipynb
│       ├── dcf_valuation.ipynb
│       ├── credit_analysis.ipynb
│       ├── covenant_monitoring.ipynb
│       ├── normalization_and_adjustments.ipynb
│       └── debt_waterfall.ipynb
│
├── 05_portfolio_and_scenarios/
│   ├── 11_portfolio_construction_and_valuation.ipynb
│   ├── 12_scenarios_and_stress_testing.ipynb
│   └── scenarios/
│       ├── rate_scenarios.ipynb
│       ├── credit_scenarios.ipynb
│       ├── composite_stress_tests.ipynb
│       └── scenario_impact_analysis.ipynb
│
├── 06_advanced_quant/
│   ├── 13_monte_carlo_simulation.ipynb
│   ├── 14_correlation_and_credit_models.ipynb
│   ├── 15_margin_collateral_and_xva.ipynb
│   ├── monte_carlo/
│   │   ├── stochastic_processes.ipynb
│   │   ├── discretization_schemes.ipynb
│   │   ├── exotic_payoffs_and_pricers.ipynb
│   │   └── black_scholes_benchmarks.ipynb
│   └── correlation/
│       ├── portfolio_default_simulation.ipynb
│       ├── clo_tranche_modeling.ipynb
│       └── recovery_modeling.ipynb
│
└── 07_capstone/
    └── 16_end_to_end_credit_portfolio_workflow.ipynb
```

**Total**: 16 curriculum notebooks + 38 deep-dive notebooks = 54 notebooks.

---

## Notebook Specifications

### Level 1: Foundations

#### `01_core_types_and_money.ipynb`

**Module**: `finstack.core.types`, `finstack.core.currency`, `finstack.core.money`, `finstack.core.config`

**API coverage**:
- `Currency(code)`, `Currency.from_numeric(n)`, properties: `code`, `numeric`, `decimals`, `to_json()`, `from_json()`
- ISO constant aliases: `USD`, `EUR`, `GBP`, `JPY`, etc.
- `Money.try_new(amount, currency)`, `Money.zero(currency)`, `format()`, `to_tuple()`, `from_tuple()`, arithmetic (`+`, `-`), cross-currency error
- `Rate(value)`, `Bps(value)`, `Percentage(value)`
- `CreditRating.from_name(name)`, `CurveId(id)`, `InstrumentId(id)`, `Attributes` (get/set/keys)
- `FinstackConfig`, `RoundingMode.from_name()`, `ToleranceConfig`

**Mini-example**: Build a multi-currency cash position tracker — create currencies, Money amounts, convert display formats, demonstrate arithmetic and error handling.

---

#### `02_dates_calendars_schedules.ipynb`

**Module**: `finstack.core.dates`

**API coverage**:
- `create_date(y, m, d)`, `days_since_epoch()`, `date_from_epoch_days()`
- `DayCount.from_name()` — ACT/360, ACT/365, 30/360, ACT/ACT-ISDA, etc.
- `DayCountContext`, `DayCountContextState`, `Thirty360Convention`
- `TenorUnit`, `Tenor.parse(s)`, tenor presets, `from_payments_per_year()`
- `PeriodKind`, `PeriodId(s)`, `Period`, `PeriodPlan`, `FiscalConfig`
- `build_periods(range_str)`, `build_fiscal_periods()`
- `BusinessDayConvention`, `CalendarMetadata`, `HolidayCalendar`
- `adjust(date, calendar, convention)`, `available_calendars()`
- `StubKind`, `ScheduleErrorPolicy`, `Schedule`
- `ScheduleBuilder` fluent: `.frequency()`, `.stub_rule()`, `.adjust_with()`, `.end_of_month()`, `.cds_imm()`, `.imm()`, `.error_policy()`, `.build()`

**Mini-example**: Generate a semi-annual bond payment schedule with holiday adjustment for a 5-year USD corporate bond, showing stub handling and business day conventions.

---

#### `03_market_data_and_curves.ipynb`

**Module**: `finstack.core.market_data`

**API coverage** (high-level orchestration):
- `DiscountCurve` — brief construction, `get_discount()`, `get_forward_rate()`
- `ForwardCurve`, `HazardCurve`, `PriceCurve`, `VolatilityIndexCurve` — one example each
- `FxConversionPolicy`, `FxRateResult`, `FxMatrix` — `set_quote()`, `rate()`
- `MarketContext` — `insert()` chaining, `insert_fx()`, getters: `get_discount()`, `get_forward()`, `get_hazard()`, `get_price_curve()`, `get_vol_index_curve()`, `fx()`

**Mini-example**: Assemble a complete `MarketContext` for pricing a USD corporate bond — OIS discount curve, credit hazard curve, and FX rate to EUR. Cross-reference deep dives for detailed curve construction.

---

#### `04_math_toolkit.ipynb`

**Module**: `finstack.core.math`

**API coverage**:
- `linalg`: `cholesky_decomposition(matrix, n)`, `cholesky_solve(L, b, n)`, `validate_correlation_matrix(matrix, n)`
- `stats`: `mean(values)`, `variance(values)`, `population_variance(values)`, `correlation(x, y)`, `covariance(x, y)`, `quantile(values, q)`
- `special_functions`: `norm_cdf(x)`, `norm_pdf(x)`, `standard_normal_inv_cdf(p)`, `erf(x)`, `ln_gamma(x)`
- `summation`: `kahan_sum(values)`, `neumaier_sum(values)`

**Mini-example**: Use Cholesky decomposition to generate correlated random variables, verify with stats functions, and demonstrate numerical stability of Kahan summation vs naive sum.

---

### Level 1 Deep Dives: `market_data/`

#### `discount_curves.ipynb`

- Construction from tenor/DF pillar pairs
- `get_discount(date)` at arbitrary dates, interpolation behavior
- Implied forward rates between two dates
- Multi-curve: OIS vs SOFR with different `CurveId`s
- Practical: build USD OIS and EUR ESTR curves

#### `forward_curves.ipynb`

- Construction from forward rate pillars
- Extracting projected rates at future dates
- Relationship to discount curves
- Use cases: commodity forwards, projected floating rates

#### `hazard_curves.ipynb`

- Construction from survival probability pillars
- Survival probability extraction, implied hazard rates
- Credit term structure (flat vs upward-sloping)
- Practical: IG vs HY issuer curves

#### `price_curves.ipynb`

- Construction from price/date pillars
- Commodity and equity forward price extraction

#### `volatility_index_curves.ipynb`

- Construction from vol/date pillars
- Term structure of implied vol

#### `vol_surfaces.ipynb`

- Strike/expiry grid construction
- Smile and skew interpretation
- Vol extraction at arbitrary strike/expiry
- Equity vs rate vs FX vol surfaces

#### `fx_matrix.ipynb`

- `FxMatrix` with `set_quote()`, direct/indirect quotes
- Triangulation via vehicle currency
- `FxConversionPolicy`, `FxRateResult`
- Multi-currency setup: USD, EUR, GBP, JPY

#### `inflation_curves.ipynb`

- InflationCurve construction
- Breakeven inflation rates
- Seasonal patterns
- Cross-ref to instruments/inflation_linked.ipynb

### Level 1 Deep Dives: `dates/`

#### `day_count_conventions.ipynb`

- All DayCount conventions with year fraction comparison table
- When to use which: ACT/360 for money markets, 30/360 for corporate bonds, etc.
- `DayCountContext` for managing state

#### `holiday_calendars.ipynb`

- `available_calendars()` listing
- `HolidayCalendar` construction, `CalendarMetadata`
- `adjust(date, calendar, convention)` with all `BusinessDayConvention` variants
- Multi-calendar adjustment chains

#### `schedule_building.ipynb`

- `ScheduleBuilder` fluent API deep dive
- Stub handling: short/long front/back stubs
- CDS IMM date schedules
- End-of-month conventions
- `ScheduleErrorPolicy` options
- Complex real-world schedules

---

### Level 2: Instrument Pricing

#### `05_pricing_fundamentals.ipynb`

**Module**: `finstack.valuations`

**API coverage**:
- `validate_instrument_json(json)` — parse and canonicalize instrument JSON
- `list_standard_metrics()` — discover available metrics
- `price_instrument(instrument_json, market_json, as_of, model)` — basic pricing
- `price_instrument_with_metrics(instrument_json, market_json, as_of, model, metrics)` — pricing + risk
- `ValuationResult.from_json(json)` — deserialize results
- `ValuationResult` properties: `instrument_id`, `get_price`, `currency`, `get_metric(key)`, `metric_keys()`, `metric_count()`
- `ValuationResult` covenant helpers: `all_covenants_passed()`, `failed_covenants()`
- Metric naming: `bucketed_dv01::USD-OIS::10y`, `cs01::BOND_A`
- Model keys: `discounting`, `black76`, `hazard_rate`, `hull_white_1f`, `tree`, `normal`, `monte_carlo_gbm`

**Mini-example**: Price a 5-year USD corporate bond — build instrument JSON, construct MarketContext, price with discounting model, extract DV01/duration/convexity/Z-spread.

---

#### `06_pricing_across_asset_classes.ipynb`

**Module**: `finstack.valuations` (with diverse instrument JSON)

**Covers**: One concise example per asset class showing instrument JSON setup → MarketContext requirements → price → key metrics. This is a tour, not exhaustive.
- Rates: deposit, IRS
- Credit: single-name CDS with HazardCurve
- Equity: equity option with VolSurface
- FX: FX option with FxMatrix
- Exotic: barrier option via `monte_carlo_gbm` engine

Cross-references to `instruments/` sub-directory for depth on each.

---

### Level 2 Deep Dives: `instruments/`

Each instrument notebook follows: **Instrument JSON schema → MarketContext setup → Available pricing models → Full metric set → Comparison/sensitivity**.

#### `complex_cashflows.ipynb`

Foundation for bonds/loans. CashFlowBuilder: ScheduleParams, fixed/float, amortization (bullet, linear, custom), PIK (full/split/toggle), step-up coupons, caps/floors, floating rate mechanics (index, spread, resets), callability/puttability, custom programs. Polars display.

#### `bonds_and_fixed_income.ipynb`

Vanilla fixed, FRN, amortizing, PIK, step-up, callable/puttable. DV01, duration, convexity, Z-spread, OAS. Side-by-side metric table.

#### `loans_and_credit_facilities.ipynb`

Term loan A/B, revolving credit (draws, commitment fees, utilization), delayed-draw, PIK toggle from liquidity, covenant-linked spread step-ups, IRR analysis.

#### `rates_derivatives.ipynb`

Deposits, FRA, IRS (vanilla, basis, cross-ccy), futures, swaptions (payer/receiver), caps/floors. PV01, bucketed DV01.

#### `credit_derivatives.ipynb`

Single-name CDS (HazardCurve, CS01, survival), CDS index, CDS tranche (base correlation, attachment/detachment), CDS options. Par spread repricing.

#### `equity_and_options.ipynb`

Equity + MarketScalar. European options (BS, VolSurface, Greeks). Exotics: barrier, Asian, lookback, cliquet, CMS, quanto, range accrual, autocallable. MC vs analytical comparison.

#### `fx_instruments.ipynb`

FxSpot, FxForward/FxSwap (points, CIP), FxOption (Garman-Kohlhagen, smile). Multi-currency exposure.

#### `inflation_linked.ipynb`

InflationCurve setup. Inflation-linked bonds (real yield, breakeven, index ratio). Inflation swaps (zero-coupon, year-on-year).

#### `structured_credit.ipynb`

ABS, CLO, CMBS, RMBS via JSON `StructuredCredit`. Pool-level cashflows, prepayment/default/loss. Tranche waterfall, OC/IC tests. Tranche metrics.

#### `convertible_bonds.ipynb`

ConvertibleBond JSON: conversion specs, ratio/price. Equity + vol inputs alongside credit/rates. Conversion premium, parity, delta, bond floor.

#### `repo_and_financing.ipynb`

Repo JSON: collateral spec, haircuts, term vs overnight. Implied repo rate, financing cost.

#### `total_return_and_variance_swaps.ipynb`

Equity TRS (reference asset, financing leg, schedule). FI index TRS. Variance swaps (realized vs implied, vega notional).

---

### Level 3: Performance & Risk Analytics

#### `07_performance_analytics.ipynb`

**Module**: `finstack.analytics`

**API coverage**:
- `Performance(prices_df)` and `Performance.from_arrays(dates, prices, names)`
- `reset_date_range()`, `reset_bench_ticker()`
- Properties: `ticker_names`, `benchmark_idx`, `freq`, `uses_log_returns`, `dates()`
- Return metrics: `cagr()`, `mean_return()`, `geometric_mean()`, `cumulative_returns()`
- Risk-adjusted: `sharpe()`, `sortino()`, `calmar()`, `omega_ratio()`, `treynor()`, `gain_to_pain()`, `martin_ratio()`, `recovery_factor()`, `pain_ratio()`, `modified_sharpe()`, `sterling_ratio()`, `burke_ratio()`, `m_squared()`
- Drawdown: `max_drawdown()`, `max_drawdown_duration()`, `drawdown_series()`, `drawdown_details()`, `ulcer_index()`, `pain_index()`, `cdar()`
- Rolling: `rolling_sharpe()`, `rolling_sortino()`, `rolling_volatility()`
- Period: `period_stats()`, `lookback_returns()`
- Correlation: `correlation_matrix()`
- Outperformance: `cumulative_returns_outperformance()`, `drawdown_outperformance()`
- Standalone: `simple_returns()`, `clean_returns()`, `excess_returns()`, `convert_to_prices()`, `rebase()`, `comp_sum()`, `comp_total()`, `cagr()`, `mean_return()`, `volatility()`, `sharpe()`, `sortino()`, `downside_deviation()`, `geometric_mean()`, `omega_ratio()`, `gain_to_pain()`

**Mini-example**: Analyze a 3-ticker equity portfolio (SPY, QQQ, BND) — build Performance from synthetic price data, compute all key metrics, display rolling Sharpe, identify worst drawdown episodes.

---

#### `08_risk_and_factor_analytics.ipynb`

**Module**: `finstack.analytics`

**API coverage**:
- VaR: `value_at_risk()`, `parametric_var()`, `cornish_fisher_var()`, `expected_shortfall()`
- Benchmark: `BenchmarkAlignmentPolicy` (`.zero_on_missing()`, `.error_on_missing()`), `align_benchmark()`, `align_benchmark_with_policy()`
- Factor: `calc_beta()` → `BetaResult`, `greeks()` → `GreeksResult`, `rolling_greeks()` → `RollingGreeks`
- Multi-factor: `multi_factor_greeks()` → `MultiFactorResult`
- Relative: `tracking_error()`, `information_ratio()`, `r_squared()`, `up_capture()`, `down_capture()`, `capture_ratio()`, `batting_average()`
- Higher moments: `skewness()`, `kurtosis()`, `tail_ratio()`, `outlier_win_ratio()`, `outlier_loss_ratio()`
- Ruin: `RuinDefinition` (`.wealth_floor()`, `.terminal_floor()`, `.drawdown_breach()`), `RuinModel`, `estimate_ruin()` → `RuinEstimate`
- Grouping: `group_by_period()`, `period_stats()`, `count_consecutive()`
- Drawdown details: `to_drawdown_series()`, `drawdown_details()` → `DrawdownEpisode`, `avg_drawdown()`, `average_drawdown()`, `max_drawdown()`, `max_drawdown_from_returns()`, `max_drawdown_duration()`

**Mini-example**: Risk report for a hedge fund — 3-factor regression (market, size, value), VaR/ES comparison across methods, capture ratios vs benchmark, ruin probability estimation.

---

### Level 4: Financial Statement Modeling

#### `09_statement_modeling.ipynb`

**Module**: `finstack.statements`

**API coverage**:
- `ModelBuilder(id)` — fluent: `.periods(range, actuals_until)`, `.value(node_id, values)`, `.compute(node_id, formula)`, `.build()` → `FinancialModelSpec`
- `FinancialModelSpec` — `.from_json()`, `.to_json()`, `.id`, `.period_count`, `.node_count`, `.node_ids()`, `.has_node()`, `.schema_version`
- `Evaluator()` — `.evaluate(model)` → `StatementResult`
- `StatementResult` — `.from_json()`, `.to_json()`, `.get(node_id, period)`, `.get_node(node_id)`, `.node_ids()`, `.node_count`, `.num_periods`, `.eval_time_ms`, `.warning_count`, `.to_polars_long()`, `.to_polars_wide()`
- DSL: `parse_formula(formula)`, `validate_formula(formula)`
- `ForecastMethod` (`.forward_fill()`, `.growth_pct()`, `.curve_pct()`, `.normal()`, `.log_normal()`, `.override_method()`, `.time_series()`, `.seasonal()`)
- `NodeType` (`.value()`, `.calculated()`, `.mixed()`)
- `NodeId(id)`, `NumericMode.float64()`
- `NormalizationConfig(target_node)` — `.from_json()`, `.to_json()`, `.target_node`, `.adjustment_count`
- `normalize(results, config)`, `normalize_to_dicts(results, config)`

**Mini-example**: Build a simple P&L model (revenue, COGS, gross profit, OpEx, EBITDA) for 4 quarters with 2 actuals and 2 forecast periods. Evaluate and display as Polars wide table.

---

#### `10_statement_analytics.ipynb`

**Module**: `finstack.statements_analytics`

**API coverage**:
- `run_sensitivity(model_json, config_json)` — sensitivity analysis
- `generate_tornado_entries(result_json, metric_node, period)` — tornado charts
- `run_variance(base_json, comparison_json, config_json)` — variance analysis
- `evaluate_scenario_set(model_json, scenario_set_json)` — scenario evaluation
- `run_monte_carlo(model_json, config_json)` — MC on statements
- `backtest_forecast(actual, forecast)` — MAE/MAPE/RMSE
- `goal_seek(model_json, target_node, target_period, target_value, driver_node, driver_period, ...)` — solver
- `evaluate_dcf(model_json, wacc, terminal_value_json, ...)` — DCF valuation
- `run_corporate_analysis(model_json, ...)` — full corporate analysis
- `pl_summary_report(results_json, line_items, periods)` — formatted P&L
- `credit_assessment_report(results_json, as_of)` — credit report
- Dependency: `trace_dependencies()`, `trace_dependencies_detailed()`, `direct_dependencies()`, `all_dependencies()`, `dependents()`
- Explain: `explain_formula()`, `explain_formula_text()`

**Mini-example**: Take the model from notebook 09, run sensitivity on revenue growth, generate tornado entries for EBITDA, goal-seek the revenue that produces a target EBITDA, trace dependencies.

---

### Level 4 Deep Dives: `models/`

#### `three_statement_model.ipynb`

Full linked P&L, BS, CF. Revenue → COGS → OpEx → D&A → interest → taxes. Working capital (AR, AP, inventory). CapEx, debt, equity. Cross-statement linkages via DSL. Forecast methods. `to_polars_wide` output.

#### `lbo_analysis.ipynb`

Sources/uses, entry EV/EBITDA. Debt schedule (TL, revolver, mezz/PIK). Operating model. Amort + ECF sweep (`WaterfallSpec`, `EcfSweepSpec`). PIK toggle (`PikToggleSpec`). Exit IRR/MOIC. Sensitivity tables.

#### `dcf_valuation.ipynb`

UFCF projection model. `evaluate_dcf` with MarketContext discount curve. Terminal value (perpetuity growth vs exit multiple). WACC construction. EV-to-equity bridge. Sensitivity: WACC vs terminal growth.

#### `credit_analysis.ipynb`

`run_corporate_analysis` end-to-end. Leverage (Debt/EBITDA), coverage (IC, FCCR, DSCR), liquidity. `credit_assessment_report`, `pl_summary_report`. Scenario overlays via `evaluate_scenario_set`.

#### `covenant_monitoring.ipynb`

Covenant definition (maintenance vs incurrence). Springing conditions. Basket headroom. `forecast_covenant` with `CovenantForecastConfig`. Multi-scenario projection.

#### `normalization_and_adjustments.ipynb`

`NormalizationConfig`, `normalize`, `normalize_to_dicts`. Add-backs, percentage fees, capped items. Raw → adjusted EBITDA audit trail.

#### `debt_waterfall.ipynb`

Capital structure priority. `WaterfallSpec`, `EcfSweepSpec`, `PikToggleSpec`. `Evaluator.evaluate_with_market_context`. Period-by-period allocation. Stress testing.

---

### Level 5: Portfolio & Scenario Management

#### `11_portfolio_construction_and_valuation.ipynb`

**Module**: `finstack.portfolio`

**API coverage**:
- Portfolio spec JSON (entities, instruments, positions)
- `parse_portfolio_spec(json)`, `build_portfolio_from_spec(json)`
- `value_portfolio(spec_json, market_json, strict_risk)` — valuation
- `portfolio_result_total_value(result_json)`, `portfolio_result_get_metric(result_json, metric_id)`
- `aggregate_metrics(valuation_json, base_ccy, market_json, as_of)`
- `aggregate_cashflows(spec_json, market_json)` — cashflow ladder

**Mini-example**: Build a 5-instrument credit portfolio (3 bonds, 1 CDS, 1 deposit), value against MarketContext, aggregate metrics, display cashflow ladder.

---

#### `12_scenarios_and_stress_testing.ipynb`

**Module**: `finstack.scenarios`, `finstack.portfolio`

**API coverage**:
- `list_builtin_templates()`, `list_builtin_template_metadata()`
- `build_from_template(template_id)`, `list_template_components(template_id)`, `build_template_component(template_id, component_id)`
- `parse_scenario_spec(json)`, `build_scenario_spec(id, operations_json, ...)`, `validate_scenario_spec(json)`
- `compose_scenarios(specs_json)` — merge multiple scenarios
- `apply_scenario_to_market(scenario_json, market_json, as_of)` — market-only
- `apply_scenario(scenario_json, market_json, model_json, as_of)` — market + model
- `apply_scenario_and_revalue(spec_json, scenario_json, market_json)` — portfolio integration

**Mini-example**: Load rate shock and credit widening templates, compose them, apply to market, revalue the portfolio from notebook 11, compute P&L impact.

---

### Level 5 Deep Dives: `scenarios/`

#### `rate_scenarios.ipynb`

Parallel shift, steepener, flattener, inversion. Build from templates and custom operations. Apply to market and show curve changes.

#### `credit_scenarios.ipynb`

Spread widening (uniform, sector-specific). Rating migration. HazardCurve shocks. Apply and show survival probability changes.

#### `composite_stress_tests.ipynb`

Compose rate + credit + FX shocks. Historical replay templates. Priority ordering. Multi-step scenario application.

#### `scenario_impact_analysis.ipynb`

`apply_scenario_and_revalue` workflow. P&L decomposition: rates contribution vs credit contribution vs FX. Report JSON parsing.

---

### Level 6: Advanced Quantitative Methods

#### `13_monte_carlo_simulation.ipynb`

**Module**: `finstack.monte_carlo`

**API coverage**:
- `TimeGrid(t_max, num_steps)`, `TimeGrid.from_times([...])`, properties
- `McEngineConfig(num_paths, seed, ...)` — `.price_call()`, `.price_put()`
- `McEngine(num_paths, time_grid, ...)` — `.price_european_call()`, `.price_european_put()`
- `GbmProcess(rate, div_yield, vol)`
- `HestonProcess(rate, div_yield, v0, kappa, theta, xi, rho)` — `.satisfies_feller`
- `EuropeanPricer(num_paths, seed)` — `.price_call()`, `.price_put()`
- `MonteCarloResult` — `.mean`, `.stderr`, `.std_dev`, `.ci_lower`, `.ci_upper`, `.num_paths`, `.relative_stderr()`
- `black_scholes_call()`, `black_scholes_put()`
- `price_european_call()`, `price_european_put()` — standalone convenience

**Mini-example**: Price a European call three ways — Black-Scholes analytical, `EuropeanPricer` MC, and `McEngine` with explicit TimeGrid. Compare results and confidence intervals.

---

#### `14_correlation_and_credit_models.ipynb`

**Module**: `finstack.correlation`

**API coverage**:
- `CopulaSpec` — `.gaussian()`, `.student_t(df)`, `.random_factor_loading(vol)`, `.multi_factor(n)`, `.build()` → `Copula`
- `Copula` — `.conditional_default_prob()`, `.num_factors`, `.model_name`, `.tail_dependence()`
- `RecoverySpec` — `.constant(rate)`, `.market_correlated(mean, vol, corr)`, `.market_standard_stochastic()`, `.build()` → `RecoveryModel`
- `RecoveryModel` — `.expected_recovery`, `.conditional_recovery()`, `.lgd`, `.conditional_lgd()`, `.is_stochastic`
- `FactorSpec` — `.single_factor(vol, mr)`, `.two_factor(ppay, credit, corr)`, `.build()` → `FactorModel`
- `SingleFactorModel`, `TwoFactorModel` (`.rmbs_standard()`, `.clo_standard()`), `MultiFactorModel` (`.uncorrelated()`, `.generate_correlated_factors()`)
- `CorrelatedBernoulli(p1, p2, corr)` — joint probs, conditionals, `sample_from_uniform(u)`
- `correlation_bounds(p1, p2)`, `joint_probabilities(p1, p2, corr)`
- `validate_correlation_matrix(matrix, n)`, `cholesky_decompose(matrix, n)`

**Mini-example**: Compare Gaussian vs Student-t copula conditional default probabilities across market factor realizations. Show tail dependence difference.

---

#### `15_margin_collateral_and_xva.ipynb`

**Module**: `finstack.margin`

**API coverage**:
- Identifiers: `NettingSetId.bilateral()`, `.cleared()`, `ClearingStatus`, `ImMethodology`, `MarginTenor`, `MarginCallType`, `CollateralAssetClass`
- CSA: `CsaSpec.usd_regulatory()`, `.eur_regulatory()`, `.from_json()`, `.to_json()`, properties
- Collateral: `EligibleCollateralSchedule` (`.cash_only()`, `.bcbs_standard()`, `.us_treasuries()`), `.is_eligible()`, `.haircut_for()`
- VM: `VmCalculator(csa)` → `.calculate(exposure, posted, ccy, y, m, d)` → `VmResult` (`.gross_exposure`, `.net_exposure`, `.delivery_amount`, `.return_amount`, `.net_margin`, `.requires_call`)
- IM: `ImResult` (`.amount`, `.currency`, `.methodology`, `.mpor_days`, `.breakdown_keys()`, `.breakdown_amount()`)
- XVA: `XvaConfig(time_grid, recovery_rate, own_recovery, funding)`, `FundingConfig(spread_bps, benefit_bps)`, `ExposureProfile`, `XvaResult` (`.cva`, `.dva`, `.fva`, `.bilateral_cva`, `.max_pfe`, `.effective_epe`, profiles), `CsaTerms`, `XvaNettingSet`
- Analytics: `MarginUtilization`, `ExcessCollateral`, `MarginFundingCost`, `Haircut01`
- `CONSTANTS` dict

**Mini-example**: Set up bilateral CSA, calculate VM for a positive exposure, check margin adequacy, compute funding cost.

---

### Level 6 Deep Dives: `monte_carlo/`

#### `stochastic_processes.ipynb`

All processes side-by-side: `GbmProcess`, `BrownianProcess`, `HestonProcess` (Feller condition), `CirProcess`, `MertonJumpProcess`, `BatesProcess`, `SchwartzSmithProcess`, `MultiGbmProcess`. Parameter interpretation, when to use which.

#### `discretization_schemes.ipynb`

`ExactGbm`, `ExactMultiGbm`, `EulerMaruyama`, `LogEuler`, `Milstein`. Convergence comparison, accuracy vs speed tradeoffs, matching scheme to process.

#### `exotic_payoffs_and_pricers.ipynb`

All payoffs: `EuropeanCall/Put`, `DigitalCall/Put`, `ForwardLong/Short`, `AsianCall/Put`, `BarrierOption`, `BasketCall/Put`, `AmericanCall/Put`.
All pricers: `EuropeanPricer`, `PathDependentPricer` (`.price_asian_call/put`), `LsmcPricer` (`.price_american_call/put`).
Choosing the right pricer for each payoff.

#### `black_scholes_benchmarks.ipynb`

`black_scholes_call/put` analytical. MC convergence study: how num_paths affects accuracy. Put-call parity verification. Greeks by finite difference bumping.

### Level 6 Deep Dives: `correlation/`

#### `portfolio_default_simulation.ipynb`

Full pipeline: individual PDs → copula → correlated defaults → portfolio loss distribution. Compare Gaussian/Student-t/RFL loss tails.

#### `clo_tranche_modeling.ipynb`

`TwoFactorModel.clo_standard()`. Base correlation. Tranche attachment/detachment. Expected tranche loss across scenarios.

#### `recovery_modeling.ipynb`

`RecoverySpec.constant()` vs `.market_correlated()` vs `.market_standard_stochastic()`. Conditional recovery/LGD across market states. Impact on portfolio loss.

---

### Level 7: Capstone

#### `16_end_to_end_credit_portfolio_workflow.ipynb`

**Modules**: All

Integrates everything into a realistic workflow:

1. **Market setup** — Build full `MarketContext` (discount, forward, hazard curves, FX, vol)
2. **Instrument pricing** — Price bonds, CDS, term loan via `price_instrument_with_metrics`
3. **Portfolio construction** — Assemble portfolio JSON, `value_portfolio`, `aggregate_metrics`
4. **Issuer modeling** — `ModelBuilder` for one issuer's P&L, `Evaluator`, `run_corporate_analysis`
5. **Stress testing** — Compose rate + credit scenarios, `apply_scenario_and_revalue`, P&L impact
6. **Performance analysis** — `Performance` from synthetic historical prices, Sharpe/VaR/drawdown
7. **Risk analytics** — Factor regression, ruin estimation
8. **Reporting** — `credit_assessment_report`, `pl_summary_report`, metric summary table

---

## Infrastructure

### `README.md`

Contents:
- Overview of the curriculum structure (levels 1-7)
- Prerequisites (Python, finstack installed, pandas, polars)
- How to run: `uv run jupyter lab` from `finstack-py/examples/notebooks/`
- How to run all notebooks programmatically: `uv run python run_all_notebooks.py`
- Directory map with one-line descriptions

### `run_all_notebooks.py`

Script that discovers and executes all `.ipynb` files using `nbclient`/`nbformat`. Reports pass/fail per notebook. Excludes none (unlike legacy which excluded slow scripts). Sets `PYTHONPATH` for in-repo finstack-py.

---

## Implementation Priority

Phase 1 (foundation — unblocks everything else):
1. Infrastructure: README.md, run_all_notebooks.py, directory structure
2. `01_core_types_and_money.ipynb`
3. `02_dates_calendars_schedules.ipynb`
4. `03_market_data_and_curves.ipynb`
5. `04_math_toolkit.ipynb`

Phase 2 (pricing and analytics — the core use cases):
6. `05_pricing_fundamentals.ipynb`
7. `06_pricing_across_asset_classes.ipynb`
8. `07_performance_analytics.ipynb`
9. `08_risk_and_factor_analytics.ipynb`

Phase 3 (modeling and portfolio):
10. `09_statement_modeling.ipynb`
11. `10_statement_analytics.ipynb`
12. `11_portfolio_construction_and_valuation.ipynb`
13. `12_scenarios_and_stress_testing.ipynb`

Phase 4 (advanced and capstone):
14. `13_monte_carlo_simulation.ipynb`
15. `14_correlation_and_credit_models.ipynb`
16. `15_margin_collateral_and_xva.ipynb`
17. `16_end_to_end_credit_portfolio_workflow.ipynb`

Phase 5 (deep dives — in any order, can be parallelized):
18. All `market_data/` notebooks
19. All `dates/` notebooks
20. All `instruments/` notebooks
21. All `models/` notebooks
22. All `scenarios/` notebooks
23. All `monte_carlo/` notebooks
24. All `correlation/` notebooks
